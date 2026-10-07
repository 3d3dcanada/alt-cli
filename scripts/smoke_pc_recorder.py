#!/usr/bin/env python3
"""Actual Alt/Goose PC recorder journeys with weight-free provider fixtures.

Good and deliberately incorrect repairs must receive different correctness
outcomes. Both adapters are tested. No fixture is counted as a model trial.
"""
import argparse
import contextlib
import hashlib
import json
import re
import subprocess
import tempfile
import threading
import time
from http.server import ThreadingHTTPServer
from pathlib import Path
from smoke_goose import Provider as BaseProvider

MODEL = 'pc-recorder-protocol-fixture'
BAD = False
REQUESTS = []
CORRECT = "def greet(name):\n    return 'Hello, ' + (name.strip() or 'friend') + '!'\n"


class Provider(BaseProvider):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        if self.path == '/api/show':
            return self.send_json({'capabilities': ['completion', 'tools'], 'model_info': {'llama.context_length': 8192}})
        assert self.path == '/v1/chat/completions' and body['model'] == MODEL
        REQUESTS.append(body)
        latest = next(str(m['content']) for m in reversed(body['messages']) if m['role'] == 'user')
        self.send_response(200)
        self.send_header('Content-Type', 'text/event-stream')
        self.end_headers()
        if 'Do not edit files or execute commands for this message.' in latest:
            try:
                self.wfile.write(b'data: {"choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}\n\n')
                self.wfile.flush()
                time.sleep(2)
            except (BrokenPipeError, ConnectionResetError):
                pass
            return
        tools = {t['function']['name'].split('__')[-1]: t['function']['name'] for t in body['tools']}
        results = [str(m['content']) for m in body['messages'] if m['role'] == 'tool']
        if not results:
            tool, args = 'read', {'path': 'greeting.py'}
        elif 'Applied checkpoint' in results[-1]:
            tool, args = 'run_check', {'name': 'Practice behavior'}
        elif 'Current file greeting.py' in results[-1] and 'Evidence ID:' not in results[-1]:
            handle = re.search(r'handle (h\d+-[a-f0-9]+):', results[-1])[1]
            tool, args = 'edit_text', {'path': 'greeting.py', 'handle': handle, 'new_text': "def greet(name):\n    return 'broken'\n" if BAD else CORRECT}
        else:
            tool = None
        if tool:
            message = {'role': 'assistant', 'content': None, 'tool_calls': [{'index': 0, 'id': 'pc-' + str(len(REQUESTS)), 'type': 'function', 'function': {'name': tools[tool], 'arguments': json.dumps(args)}}]}
            finish = 'tool_calls'
        else:
            message = {'role': 'assistant', 'content': 'Completed fixture response; actual check evidence determines correctness.'}
            finish = 'stop'
        chunk = {'id': 'pc-fixture', 'model': MODEL, 'choices': [{'index': 0, 'delta': message, 'finish_reason': finish}], 'usage': {'prompt_tokens': 200, 'completion_tokens': 40}}
        try:
            self.wfile.write(('data: ' + json.dumps(chunk) + '\n\ndata: [DONE]\n\n').encode())
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass


def main():
    global BAD
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--alt', type=Path, default=Path('target/debug/alt'))
    parser.add_argument('--engine', type=Path, required=True)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    server = ThreadingHTTPServer(('127.0.0.1', 0), Provider)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    rows = []
    try:
        with (contextlib.nullcontext(str(args.output.parent / ('pc-recorder-raw-' + str(time.time_ns())))) if args.output else tempfile.TemporaryDirectory(prefix='alt-pc-recorder-')) as directory:
            root = Path(directory)
            root.mkdir(parents=True,exist_ok=True)
            for provider, bad in [('openai', False), ('ollama', False), ('openai', True)]:
                BAD = bad
                state, project = root / (provider + str(bad)), root / 'original'
                project.mkdir(exist_ok=True)
                (project / 'keep.py').write_text('USER_PROJECT_UNCHANGED')
                endpoint = 'http://127.0.0.1:{}'.format(server.server_port) + ('/v1' if provider == 'openai' else '')
                base = [str(args.alt.resolve()), '--data-dir', str(state), '--engine', str(args.engine.resolve()), '--access', 'trusted']
                subprocess.run(base + ['init', '--model', MODEL, '--provider', provider, '--endpoint', endpoint, '--uncensored', '--context', '8192'], cwd=project, check=True, capture_output=True)
                (state / 'preferences.toml').write_text('project={}\naccess_policy="trusted"\ntool_profile="compact"\nworkflow="host"\nengine_path={}\n'.format(json.dumps(str(project)), json.dumps(str(args.engine.resolve()))))
                output = root / ('report-' + provider + str(bad))
                result = subprocess.run(['python3', str(Path(__file__).with_name('test-my-pc.py')), '--alt', str(args.alt.resolve()), '--data-dir', str(state), '--engine', str(args.engine.resolve()), '--live', '--repair-only', '--repair-timeout', '30', '--output', str(output)], capture_output=True, text=True, timeout=180)
                report = json.loads((output / 'summary.json').read_text())
                if bad:
                    assert result.returncode == 1 and not report['passed'], result.stdout + result.stderr
                    assert report['outcomes']['native_transport']['passed'] and not report['outcomes']['model_correctness']['passed']
                else:
                    assert result.returncode == 0 and report['passed'], result.stdout + result.stderr + str(output)
                assert (project / 'keep.py').read_text() == 'USER_PROJECT_UNCHANGED'
                assert report['repair_status'].startswith('Performed')
                assert any(c['name'] == 'normal-settings-unchanged' and c['passed'] for c in report['checks'])
                assert any(c['name'] == 'original-assertions-unchanged' and c['passed'] for c in report['checks'])
                rows.append(dict(provider=provider, incorrect_fixture=bad, expected_outcome_observed=True, recorder_passed=report['passed'], outcomes=report['outcomes']))
                print('PASS: {} {} repair, continuation, cancel/resume, assertions and settings'.format(provider, 'incorrect' if bad else 'correct'), flush=True)
    finally:
        server.shutdown()
        server.server_close()
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(dict(schema=1, binary_sha256=hashlib.sha256(args.alt.read_bytes()).hexdigest(), passed=all(r['expected_outcome_observed'] for r in rows), attempts=rows, scope='Weight-free fixtures with actual Alt/Goose and PC recorder; no measured model intelligence'), indent=2) + '\n')


if __name__ == '__main__':
    main()
