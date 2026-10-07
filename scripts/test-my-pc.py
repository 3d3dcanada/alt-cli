#!/usr/bin/env python3
"""Record local hardware, native tools, a real repair and recovery.

Select your own uncensored model in Alt first. --live reuses that exact selection
in a new test state; no weights are downloaded or copied and normal settings are
unchanged. Reports stay local; the selected endpoint receives model requests. Full access retains ordinary host permissions.
Python 3.8+ and an installed Alt/engine are required; no Python packages are needed.
"""
import argparse
import datetime
import hashlib
import json
import os
import platform
import shutil
import signal
import subprocess
import time
from pathlib import Path


FOLLOWUP = 'Continue the original greeting repair. Read the current file and run Practice behavior again. Preserve the original interface and requirements. Report the actual check result and remaining failures.'
CANCEL_PROMPT = 'Inspect the current greeting implementation and explain its behavior in detail. Do not edit files or execute commands for this message.'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None


def normal_turn(path):
    try:
        events = [json.loads(line) for line in path.read_text().splitlines()]
    except (OSError, ValueError):
        return False
    return any(e.get('type') == 'turn_end' and e.get('data', {}).get('stopReason') == 'end_turn' for e in events)


class Recorder:
    def __init__(self, binary, root, base, report):
        self.binary, self.root, self.base, self.report = binary, root, base, report

    def save(self):
        temporary = self.root / 'summary.json.tmp'
        temporary.write_text(json.dumps(self.report, indent=2) + '\n')
        temporary.replace(self.root / 'summary.json')

    def observation(self, name, passed, category='application', **details):
        entry = dict(name=name, passed=bool(passed), category=category, **details)
        self.report['checks'].append(entry)
        self.save()
        print('{}: {}'.format(name, 'PASS' if passed else 'NEEDS ATTENTION'), flush=True)
        return entry

    def run(self, name, arguments, timeout=60, cwd=None, base=None,
            expected_exit=0, category='application', cancel_state=None, env=None):
        start = time.monotonic()
        entry = dict(name=name, passed=False, category=category, command_arguments=arguments)
        child = None
        receipts = set(cancel_state.glob('inference/*/*-receipt.json')) if cancel_state else set()
        try:
            with (self.root / (name + '.stdout')).open('w') as out, (self.root / (name + '.stderr')).open('w') as err:
                child = subprocess.Popen((self.base if base is None else base) + arguments,
                                         cwd=cwd, stdout=out, stderr=err,
                                         start_new_session=True, env=env)
                deadline = start + timeout
                if cancel_state:
                    while child.poll() is None and time.monotonic() < deadline:
                        issued = set(cancel_state.glob('inference/*/*-receipt.json')) - receipts
                        if issued:
                            child.send_signal(signal.SIGINT)
                            entry['interrupt_after_issued_request'] = True
                            deadline = min(deadline, time.monotonic() + 15)
                            break
                        time.sleep(.025)
                code = child.wait(timeout=max(.01, deadline - time.monotonic()))
                entry.update(exit=code, passed=(code == expected_exit) if expected_exit is not None else code != 0)
                if cancel_state:
                    entry['passed'] = entry['passed'] and entry.get('interrupt_after_issued_request', False) and not normal_turn(self.root / (name + '.stdout'))
                if '--json' in arguments and 'run' in arguments and not cancel_state:
                    entry['normal_turn'] = normal_turn(self.root / (name + '.stdout'))
                    entry['passed'] = entry['passed'] and entry['normal_turn']
        except subprocess.TimeoutExpired:
            entry['error'] = 'Timed out; stopping the owned test process'
            if child and child.poll() is None:
                child.send_signal(signal.SIGINT)
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                    entry['forced_signal_fallback'] = True
        except KeyboardInterrupt:
            entry['error'] = 'Recorder interrupted by user'
            if child and child.poll() is None:
                child.send_signal(signal.SIGINT)
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                    entry['forced_signal_fallback'] = True
            raise
        except Exception as error:
            entry['error'] = str(error)
        finally:
            entry['seconds'] = round(time.monotonic() - start, 3)
            self.report['checks'].append(entry)
            self.save()
            print('{}: {}'.format(name, 'PASS' if entry['passed'] else 'NEEDS ATTENTION'), flush=True)
        return entry

    def read(self, name):
        return json.loads((self.root / (name + '.stdout')).read_text())

    def repair(self, timeout):
        prepared = self.run('repair-prepare', ['practice', '--isolated-state', str(self.root / 'repair-state')])
        if not prepared['passed']:
            self.report['repair_status'] = 'Unperformed: preparation failed; inspect repair-prepare.stderr'
            return
        lesson = self.read('repair-prepare')
        state, project = Path(lesson['state']), Path(lesson['project'])
        self.report['selection'] = lesson['selection']
        # Only test state and the newly created practice project receive writes.
        base = [str(self.binary), '--data-dir', str(state), '--profile', lesson['selected_profile']]
        initial = {p.name: p.read_bytes() for p in project.iterdir() if p.is_file()}
        assertion = project.parent / 'assertion.py'
        independent = self.root / 'independent-assertion.py'
        independent.write_bytes(assertion.read_bytes())
        assertion_hash = digest(independent)

        def invoke(name, args, **kwargs):
            return self.run(name, args, base=base, cwd=project, **kwargs)

        def oracle(name, expected):
            evidence = self.root / (name + '-report.json')
            entry = self.run(name, [str(independent)], base=[shutil.which('python3') or 'python3'], cwd=project,
                             env=dict(os.environ, ALT_CHECK_REPORT=str(evidence)), expected_exit=expected,
                             category='model_correctness' if expected == 0 else 'application')
            payload = json.loads(evidence.read_text()) if evidence.is_file() else {}
            tests = payload.get('tests', [])
            expected_failed = 0 if expected == 0 else 3
            correct = payload.get('schema') == 1 and payload.get('complete') is True and len(tests) == 4 and sum(t.get('status') == 'failed' for t in tests) == expected_failed
            self.observation(name + '-four-cases', correct and entry['passed'], entry['category'], executed=len(tests), failed=sum(t.get('status') == 'failed' for t in tests))
            return entry['passed'] and correct

        invoke('seed-check', ['task', 'check', 'Practice behavior'], expected_exit=1)
        oracle('seed-independent', 1)
        repaired = invoke('repair-turn', ['run', lesson['goal'], '--allow-tools', '--json', '--timeout', str(timeout)], timeout=timeout + 180, category='native_transport')
        session = None
        try:
            session = next(json.loads(line)['session']['id'] for line in (self.root / 'repair-turn.stdout').read_text().splitlines() if json.loads(line).get('type') == 'session')
        except (ValueError, StopIteration, KeyError):
            pass
        first_pass = oracle('repair-independent', 0)
        invoke('repair-verification', ['task', 'verify', '--run'], category='model_correctness', timeout=120)
        exported = invoke('repair-export', ['task', 'export'])
        changes = self.read('repair-export').get('changes', []) if exported['passed'] else []
        self.observation('native-tracked-repair', repaired['passed'] and any(c.get('status') == 'applied' for c in changes), 'native_transport')

        if session:
            before = set(state.glob('inference/*/*-request.json'))
            invoke('followup-turn', ['run', FOLLOWUP, '--resume', session, '--allow-tools', '--json', '--timeout', str(timeout)], timeout=timeout + 180, category='native_transport')
            forwarded = []
            for path in sorted(set(state.glob('inference/*/*-request.json')) - before):
                body = json.loads(path.read_text())
                forwarded.append(any(lesson['goal'] in str(m.get('content', '')) for m in body.get('messages', []) if m.get('role') == 'user'))
            self.observation('original-goal-forwarded', bool(forwarded) and all(forwarded), requests=len(forwarded))
            second_pass = oracle('followup-independent', 0)
            invoke('followup-verification', ['task', 'verify', '--run'], category='model_correctness', timeout=120)
            readonly_before = {name: (project / name).read_bytes() if (project / name).is_file() else None for name in initial}
            invoke('cancel-turn', ['run', CANCEL_PROMPT, '--resume', session, '--json', '--timeout', str(timeout)], timeout=timeout + 180, cancel_state=state, expected_exit=None, category='recovery')
            self.observation('cancel-preserves-files', all((project / name).is_file() and (project / name).read_bytes() == body for name, body in readonly_before.items()), 'recovery')
            invoke('resume-after-cancel', ['run', FOLLOWUP, '--resume', session, '--allow-tools', '--json', '--timeout', str(timeout)], timeout=timeout + 180, category='recovery')
            third_pass = oracle('resumed-independent', 0)
            invoke('resumed-verification', ['task', 'verify', '--run'], category='model_correctness', timeout=120)
            saved = invoke('saved-session', ['export', session])
            history = [json.loads(line) for line in (self.root / 'saved-session.stdout').read_text().splitlines()] if saved['passed'] else []
            self.observation('persisted-original-goal', lesson['goal'] in json.dumps(history), 'recovery')
            self.report['repair_outcomes'] = dict(first=first_pass, followup=second_pass, after_cancel=third_pass)
        else:
            self.observation('session-required-for-recovery', False, 'recovery', error='No session receipt; follow-up, cancellation and resume are unperformed')

        final_export = invoke('final-task-export', ['task', 'export'])
        payload = self.read('final-task-export') if final_export['passed'] else {}
        tasks = list(dict.fromkeys(c.get('task') for c in payload.get('changes', []) if c.get('status') == 'applied' and c.get('task')))
        for index, task in enumerate(reversed(tasks)):
            invoke('undo-' + str(index + 1), ['task', 'undo-all', task], category='recovery')
        self.observation('undo-restores-seed', bool(tasks) and all((project / name).is_file() and (project / name).read_bytes() == body for name, body in initial.items()), 'recovery')
        invoke('undo-verification', ['task', 'verify'], expected_exit=1, category='recovery')
        oracle('undo-independent', 1)
        self.observation('original-assertions-unchanged', digest(assertion) == assertion_hash and digest(independent) == assertion_hash)
        source = Path(lesson['source_state'])
        actual = {key: digest(source / name) for key, name in [('config_sha256', 'config.toml'), ('preferences_sha256', 'preferences.toml'), ('runtime_sha256', 'runtime.toml')]}
        self.observation('normal-settings-unchanged', actual == lesson['source_settings'])
        self.report['repair_status'] = 'Performed in disposable state; inspect separate correctness, transport and recovery outcomes'


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--alt', default='alt')
    parser.add_argument('--data-dir', type=Path, help='Your existing Alt state folder')
    parser.add_argument('--profile', help='Explicit existing model profile')
    parser.add_argument('--engine', type=Path, help='Existing engine, if not configured')
    parser.add_argument('--live', action='store_true', help='Run your selected uncensored model, including the disposable repair/recovery test')
    parser.add_argument('--repair-only', action='store_true', help='With --live, run repair/recovery instead of the longer context/native qualification suite')
    parser.add_argument('--repair-timeout', type=int, default=600, help='Seconds per model turn; runtime startup gets a separate 180-second margin')
    parser.add_argument('--contexts', default='2048,4096,8192')
    parser.add_argument('--output', type=Path, help='New report folder; never overwrites a run')
    args = parser.parse_args(argv)
    if args.repair_only and not args.live:
        parser.error('--repair-only requires --live')
    if not 30 <= args.repair_timeout <= 86400:
        parser.error('--repair-timeout must be 30..86400')
    try:
        contexts = [int(n) for n in args.contexts.split(',')]
        if not contexts or not all(2048 <= n <= 1048576 for n in contexts):
            raise ValueError()
    except ValueError:
        parser.error('Supply comma-separated contexts in 2048..1048576')
    binary = Path(shutil.which(args.alt) or args.alt).resolve()
    if not binary.is_file():
        parser.error('Install Alt or supply --alt /path/to/alt')
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    root = (args.output or Path.cwd() / ('alt-pc-results-' + stamp)).resolve()
    root.mkdir(mode=0o700, parents=True, exist_ok=False)
    base = [str(binary)]
    for name in ['data_dir', 'profile', 'engine']:
        value = getattr(args, name)
        if value:
            base += ['--' + name.replace('_', '-'), str(value)]
    report = dict(schema=2, created_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  os=platform.platform(), machine=platform.machine(), binary_sha256=digest(binary),
                  live_requested=args.live, checks=[], gpu_measurement_status="Unqualified by inventory alone; inspect sampled per-runtime VRAM in model-qualification.stdout when performed",
                  scope='Local application, hardware, transport, repair correctness and recovery evidence. Reports are not uploaded; the selected endpoint receives prompts/tool results. Full access keeps host permissions. Review raw reports before sharing.')
    recorder = Recorder(binary, root, base, report)
    interrupted = False
    try:
        recorder.run('build', ['build-info'])
        recorder.run('hardware', ['hardware'])
        if args.live:
            if not args.repair_only:
                recorder.run('connection', ['doctor'], 120)
                recorder.run('inference-settings', ['inference'])
                recorder.run('model-qualification', ['qualify', '--contexts', ','.join(map(str, contexts)), '--repeats', '3'], max(900, len(contexts) * 900), category='hardware_runtime')
                recorder.run('native-tool-use', ['evaluate'], 900, category='native_transport')
                recorder.run('native-streamed-roundtrip', ['qualify-tools', '--timeout', '600'], 660, category='native_transport')
            recorder.repair(args.repair_timeout)
        else:
            report['live_status'] = 'Unperformed. Select your uncensored model in Alt, then repeat with --live.'
    except KeyboardInterrupt:
        interrupted = True
        report['error'] = 'Recorder interrupted; unfinished stages remain unperformed'
    except Exception as error:
        report['error'] = str(error)
    categories = sorted(set(c['category'] for c in report['checks']))
    report['outcomes'] = {category: dict(passed=all(c['passed'] for c in report['checks'] if c['category'] == category), checks=sum(c['category'] == category for c in report['checks'])) for category in categories}
    report['passed'] = bool(report['checks']) and all(c['passed'] for c in report['checks']) and 'error' not in report and (not args.live or report.get('repair_status', '').startswith('Performed'))
    recorder.save()
    print('Reports saved to {}\nRead summary.json and docs/PC_TESTING.md. A normal model response alone does not verify a repair.'.format(root))
    return 130 if interrupted else 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
