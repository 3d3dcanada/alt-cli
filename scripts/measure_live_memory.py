#!/usr/bin/env python3
"""Measure selected-model recall across process restarts, noise and a source edit.

This is a three-observation continuation probe, not a coding benchmark. Answers
are fixture data stored in project memory/files, never embedded in the request.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sqlite3
import subprocess
import time

from evaluation_models import MODELS

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', type=Path, required=True)
p.add_argument('--engine', type=Path, required=True)
p.add_argument('--runtime', type=Path, required=True)
p.add_argument('--weights', type=Path, required=True)
p.add_argument('--model', choices=MODELS, required=True)
p.add_argument('--uncensored', action='store_true', required=True)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
selected = MODELS[a.model]

def digest(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

assert digest(a.weights) == selected['sha256'], 'Selected weights differ from pinned artifact'
root = a.output.resolve()
root.mkdir(parents=True, exist_ok=False)
binary = root / 'alt-under-test'
shutil.copy2(a.binary, binary)
project = root / 'project'
project.mkdir()
state = root / 'state'
state.mkdir()
preferences = f'project={json.dumps(str(project))}\ncontext_tokens=8192\nmax_turns=8\naccess_policy="review-only"\ntool_profile="inspect"\nruntime_path={json.dumps(str(a.runtime.resolve()))}\n[runtime]\nthreads=2\n'
if selected['thinking'] != 'default':
    preferences += 'thinking=' + ('true' if selected['thinking'] == 'on' else 'false') + '\n'
(state / 'preferences.toml').write_text(preferences)
base = [str(binary), '--data-dir', str(state), '--engine', str(a.engine.resolve())]

def command(args):
    return subprocess.run(base + args, cwd=project, capture_output=True, text=True, check=True, timeout=180).stdout

command(['models', 'import', str(a.weights.resolve()), '--uncensored', '--use'])
command(['task', 'pin', 'Cedar release tag is MARIGOLD_42.'])
command(['task', 'remember', 'Cedar uses grpc as its transport protocol.'])
source = project / 'cedar_settings.py'
source.write_text('cedar_inventory_port = 4317\n')
prompt = ('For Cedar, report the pinned release tag, saved transport protocol and the CURRENT '
          'inventory port in cedar_settings.py. Consult saved project memory and the current file. '
          'Respond only with a JSON object with keys release_tag, transport and port. Do not edit anything.')
rows = []
session = None
identity = {'build': json.loads(command(['build-info'])), 'binary_sha256': digest(binary),
            'weights_sha256': digest(a.weights), 'model': a.model,
            'engine_sha256': digest(a.engine), 'runtime_sha256': digest(a.runtime),
            'context': 8192, 'threads': 2, 'thinking': selected['thinking'],
            'uncensored': 'Explicitly selected publisher-labelled uncensored/abliterated artifact'}
(root / 'identity.json').write_text(json.dumps(identity, indent=2) + '\n')
for phase, port in [('baseline', 4317), ('restart_after_200_notes', 4317), ('restart_after_source_edit', 4629)]:
    if phase == 'restart_after_200_notes':
        for n in range(200):
            command(['task', 'remember', f'Unrelated layout note {n}: panel {n} has padding {n % 11}.'])
    if phase == 'restart_after_source_edit':
        source.write_text(f'cedar_inventory_port = {port}\n')
    evidence = root / phase
    evidence.mkdir()
    (evidence / 'memory-before.txt').write_text(command(['task', 'memory', prompt]))
    (evidence / 'source-before.py').write_bytes(source.read_bytes())
    (evidence / 'prompt.txt').write_text(prompt + '\n')
    args = base + ['run', prompt, '--allow-tools', '--json', '--timeout', '240']
    if session:
        args += ['--resume', session]
    begin = time.monotonic()
    with (evidence / 'turn.jsonl').open('w') as out, (evidence / 'turn.stderr').open('w') as err:
        proc = subprocess.Popen(args, cwd=project, stdout=out, stderr=err, start_new_session=True)
        try:
            proc.wait(timeout=280)
        except subprocess.TimeoutExpired:
            proc.send_signal(signal.SIGINT)
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait()
        finally:
            if proc.poll() is None:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait()
    events = [json.loads(line) for line in (evidence / 'turn.jsonl').read_text().splitlines()]
    sessions = [e['session']['id'] for e in events if e.get('type') == 'session']
    if sessions:
        session = sessions[-1]
    prose = ''.join(e.get('data', {}).get('update', {}).get('content', {}).get('text', '')
                    for e in events if e.get('data', {}).get('update', {}).get('sessionUpdate') == 'agent_message_chunk')
    expected = {'release_tag': 'MARIGOLD_42', 'transport': 'grpc', 'port': port}
    objects = []
    for match in re.finditer(r'\{', prose):
        try:
            value, _ = json.JSONDecoder().raw_decode(prose[match.start():])
            if isinstance(value, dict):
                objects.append(value)
        except json.JSONDecodeError:
            pass
    row = {'phase': phase, 'expected': expected, 'response_objects': objects,
           'passed': proc.returncode == 0 and objects == [expected], 'cli_exit': proc.returncode,
           'seconds': round(time.monotonic() - begin, 2), 'session': session, 'prose': prose}
    contexts = []
    for database in state.glob('projects/*/project.db'):
        with sqlite3.connect('file:' + str(database) + '?mode=ro', uri=True) as db:
            contexts.extend(json.loads(row[0]) for row in db.execute('SELECT payload FROM context_views'))
    (evidence / 'context-views.json').write_text(json.dumps(contexts, indent=2) + '\n')
    (evidence / 'report.json').write_text(json.dumps(row, indent=2) + '\n')
    rows.append(row)
    report = {'identity': identity, 'observations': rows, 'expected_observations': 3,
              'passed': sum(r['passed'] for r in rows),
              'scope': 'One selected-model continuation, three observations. Tests saved-memory/source recall; does not establish coding reliability or enlarge native context.'}
    (root / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    print(phase, row['passed'], row['seconds'], flush=True)
manifest = {str(path.relative_to(root)): digest(path) for path in root.rglob('*')
            if path.is_file() and (path.parent.name in [r['phase'] for r in rows] or path.name in ['identity.json', 'summary.json'])}
(root / 'RAW-SHA256.json').write_text(json.dumps(manifest, sort_keys=True, indent=2) + '\n')
raise SystemExit(0 if len(rows) == 3 and all(r['passed'] for r in rows) else 1)
