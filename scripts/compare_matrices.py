#!/usr/bin/env python3
"""Compare complete matrices only when model, runtime, harness and budgets match."""
import argparse
import hashlib
import json
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('baseline', type=Path)
p.add_argument('candidate', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
before = json.loads(a.baseline.read_text())
after = json.loads(a.candidate.read_text())
for report in [before, after]:
    assert report['complete'] and report['expected'] == report['recorded'] == 120
    assert not report['errors'] and not report['missing']
controls = [key for key in before['identity'] if key != 'binary_sha256']
assert set(controls) == set(after['identity']) - {'binary_sha256'}
for key in controls:
    assert before['identity'][key] == after['identity'][key], f'Comparison control changed: {key}'
index = lambda report: {(r['case'], r['repeat']): r for r in report['attempts']}
old = index(before)
new = index(after)
assert old.keys() == new.keys() and len(old) == 120
rows = [{'case': case, 'repeat': repeat, 'baseline_passed': old[case, repeat]['passed'],
         'candidate_passed': new[case, repeat]['passed']} for case, repeat in sorted(old)]
report = {
    'schema': 1, 'complete': True,
    'baseline_summary_sha256': hashlib.sha256(a.baseline.read_bytes()).hexdigest(),
    'candidate_summary_sha256': hashlib.sha256(a.candidate.read_bytes()).hexdigest(),
    'controls': {key: before['identity'][key] for key in controls},
    'baseline_binary_sha256': before['identity']['binary_sha256'],
    'candidate_binary_sha256': after['identity']['binary_sha256'],
    'baseline_passed': before['passed'], 'candidate_passed': after['passed'],
    'development': {'baseline': before['development'], 'candidate': after['development']},
    'held_out': {'baseline': before['held_out'], 'candidate': after['held_out']},
    'attempts': rows, 'preset_promoted': False,
    'scope': 'Descriptive counts on separate CPU runners. The same configurations and fixtures were used; random generation seeds and underlying CPU hardware were not matched. No statistical improvement or physical-GPU claim is inferred.'}
a.output.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({key: report[key] for key in ['complete', 'baseline_passed', 'candidate_passed']}))
