#!/usr/bin/env python3
"""Describe a complete retained matrix; keep manual claim review separate from behavior."""
import argparse
import collections
import hashlib
import json
import math
import statistics
from pathlib import Path
from acceptance_projects import ORACLE_VERSION
from evaluation_models import MODELS


def require(condition, message):
    if not condition:
        raise ValueError(message)


p = argparse.ArgumentParser(description=__doc__)
p.add_argument('source', type=Path, help='Directory containing summary.json and all eight shards')
p.add_argument('--model', choices=['spark', 'qwen'], required=True)
p.add_argument('--claims', type=Path, help='Manual assessments with model/case/repeat/report_sha256')
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
summary = json.loads((a.source / 'summary.json').read_text())
require(summary['complete'] and summary['recorded'] == summary['expected'] == 120, 'A complete 120-attempt matrix is required')
require(not summary['errors'] and not summary['missing'], 'Matrix has errors or missing attempts')
require(summary['identity']['sha256'] == MODELS[a.model]['sha256'], 'Selected model differs from the recorded model')
require(summary['identity']['oracle_version'] == ORACLE_VERSION, 'Oracle execution contract differs')
rows = {}
tool_statuses = collections.Counter()
failure_messages = collections.Counter()
unchanged_sources = 0
for path in sorted(a.source.glob('uncensored-matrix-*/*/report.json')):
    row = json.loads(path.read_text())
    key = (row['case'], row['repeat'])
    require(key not in rows, f'Duplicate attempt: {key}')
    rows[key] = (row, path)
expected = {(r['case'], r['repeat']): r for r in summary['attempts']}
require(rows.keys() == expected.keys() and len(rows) == 120 and len(summary['attempts']) == 120, 'Raw attempts differ from summary')
for key, (row, path) in rows.items():
    require(row['passed'] == expected[key]['passed'], f'Score differs from summary: {key}')
    manifest = json.loads((path.parent.parent / 'RAW-SHA256.json').read_text())
    require(hashlib.sha256(path.read_bytes()).hexdigest() == manifest[str(path.relative_to(path.parent.parent))], f'Raw report hash differs: {key}')
    require(row['oracle_unchanged'] and row['before']['passed'] is False, f'Invalid oracle control: {key}')
    require(row['passed'] == (row['after']['passed'] and row['after'].get('completion_observed', False)), f'Score lacks completion evidence: {key}')
    for name in ['fixture.json', 'turn.jsonl']:
        raw = path.with_name(name)
        require(hashlib.sha256(raw.read_bytes()).hexdigest() == manifest[str(raw.relative_to(path.parent.parent))], f'Raw {name} hash differs: {key}')
    fixture = json.loads(path.with_name('fixture.json').read_text())
    unchanged_sources += row['resulting_source'] == fixture['original_files']
    names = {call.get('toolCallId'): call.get('_meta', {}).get('goose', {}).get('toolCall', {}).get('toolName', call.get('title', 'unknown')) for call in row['tool_calls']}
    failed_updates = 0
    for line in path.with_name('turn.jsonl').read_text().splitlines():
        update = json.loads(line).get('data', {}).get('update', {})
        if update.get('sessionUpdate') != 'tool_call_update' or update.get('status') not in ['failed', 'completed']:
            continue
        tool = names.get(update.get('toolCallId'), 'unknown')
        tool_statuses[(tool, update['status'])] += 1
        if update['status'] == 'failed':
            failed_updates += 1
            body = '\n'.join(v.get('content', {}).get('text', '') for v in update.get('content', []))
            headline = body.splitlines()[0] if body else '<no text>'
            if headline.startswith('Evidence ID:'):
                headline = 'Check evidence indicates failure'
            failure_messages[(tool, headline)] += 1
    require(failed_updates == row['failed_tool_updates'], f'Failed update count differs: {key}')

reviews = {}
if a.claims:
    for review in json.loads(a.claims.read_text()):
        if review['model'] != a.model:
            continue
        key = (review['case'], review['repeat'])
        require(key in rows and key not in reviews, f'Unknown or repeated review: {key}')
        row, path = rows[key]
        require(review['report_sha256'] == hashlib.sha256(path.read_bytes()).hexdigest(), f'Review report hash differs: {key}')
        require(review['source_behavior_passed'] == row['passed'], f'Review behavior score differs: {key}')
        require(review['assessment'] and review['reason'], f'Review lacks an assessment or reason: {key}')
        reviews[key] = review

budget_notice = "I've reached the maximum number of actions I can do without user input. Would you like me to continue?"
records = [row for row, _ in rows.values()]
times = sorted(row['wall_seconds'] for row in records)
memory = [row['sampled_alt_tree_peak_rss_bytes'] for row in records]
report = {
    'schema': 1,
    'model': a.model,
    'summary_sha256': hashlib.sha256((a.source / 'summary.json').read_bytes()).hexdigest(),
    'recorded': len(records),
    'behavior_passed': sum(row['passed'] for row in records),
    'development': summary['development'],
    'held_out': summary['held_out'],
    'protocol_stop_reasons': dict(collections.Counter(str(row['scores']['stop_reason']) for row in records)),
    'cli_exit_codes': dict(collections.Counter(str(row['cli_exit']) for row in records)),
    'outer_timeouts': sum(row['outer_timeout'] for row in records),
    'action_budget_notices': sum(budget_notice in row['final_prose'] for row in records),
    'attempts_with_unchanged_source': unchanged_sources,
    'terminal_tool_updates': [{'tool': tool, 'status': status, 'count': count} for (tool, status), count in sorted(tool_statuses.items())],
    'failed_tool_messages': [{'tool': tool, 'headline': headline, 'count': count} for (tool, headline), count in failure_messages.most_common()],
    'wall_seconds': {'median': statistics.median(times), 'p95_nearest_rank': times[math.ceil(.95 * len(times)) - 1], 'max': max(times)},
    'sampled_process_tree_peak_rss_bytes': {'median': statistics.median(memory), 'max': max(memory)},
    'by_case': {case: {'passed': sum(row['passed'] for row in records if row['case'] == case), 'recorded': sum(row['case'] == case for row in records)} for case in sorted({row['case'] for row in records})},
    'claim_review': {'reviewed': len(reviews), 'unreviewed': len(records) - len(reviews), 'assessments': dict(collections.Counter(r['assessment'] for r in reviews.values())), 'rows': [reviews[key] for key in sorted(reviews)]},
    'preset_promoted': False,
    'scope': 'Descriptive seeded-task results on GitHub CPU runners. end_turn may include an action-budget notice; it is not behavioral completion. Timing includes the post-turn oracle. RSS is sampled, not an exact peak. Tool statuses count terminal update events, not independent attempts; failure headlines group check evidence IDs but otherwise preserve the first message line. These are descriptive frequencies, not causal diagnoses. Prose assessments are supplied by a reviewer and bound to the original report hash; this script never infers claim accuracy or promotes a preset.',
}
a.output.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({key: report[key] for key in ['model', 'recorded', 'behavior_passed', 'action_budget_notices']}))
