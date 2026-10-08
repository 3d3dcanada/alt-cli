#!/usr/bin/env python3
"""Observed request stages, finite additions and allocation recovery in a real PTY.
Uses real Goose and a weight-free provider; not a model quality measurement.
"""
import argparse
import hashlib
import json
import sqlite3
import subprocess
import tempfile
import threading
import time
import tomllib
from http.server import ThreadingHTTPServer
from pathlib import Path
from smoke_goose import Provider as BaseProvider
from terminal_harness import Terminal

MODEL = 'allowance-protocol-fixture'
REQUESTS = []


class Provider(BaseProvider):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        assert self.path == '/v1/chat/completions' and body['model'] == MODEL
        REQUESTS.append(body)
        time.sleep(.6)  # Makes the observed waiting stage visible; no fake progress.
        self.send_response(200)
        self.send_header('Content-Type', 'text/event-stream')
        self.end_headers()
        first = {'id': 'allowance', 'model': MODEL, 'choices': [{'index': 0, 'delta': {'content': 'Actual allowance fixture response.'}, 'finish_reason': None}]}
        final = {'id': 'allowance', 'model': MODEL, 'choices': [{'index': 0, 'delta': {}, 'finish_reason': 'stop'}], 'usage': {'prompt_tokens': 200, 'completion_tokens': 40}}
        try:
            self.wfile.write(('data: ' + json.dumps(first) + '\n\n').encode()); self.wfile.flush()
            time.sleep(.6)
            self.wfile.write(('data: ' + json.dumps(final) + '\n\ndata: [DONE]\n\n').encode()); self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--alt', type=Path, default=Path('target/debug/alt'))
    p.add_argument('--engine', type=Path, required=True)
    p.add_argument('--output', type=Path)
    a = p.parse_args()
    server = ThreadingHTTPServer(('127.0.0.1', 0), Provider)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    rows = []
    try:
        for width, height in [(120, 40), (80, 24), (60, 18)]:
            with tempfile.TemporaryDirectory(prefix='alt-allowance-tui-') as directory:
                root = Path(directory); state = root / 'state'; project = root / 'project'; project.mkdir()
                base = [str(a.alt.resolve()), '--data-dir', str(state)]
                subprocess.run(base + ['init', '--model', MODEL, '--endpoint', 'http://127.0.0.1:{}/v1'.format(server.server_port), '--context', '8192'], check=True, capture_output=True)
                subprocess.run(base + ['inference', '--output-tokens', '128', '--action-headroom', '64', '--generated-tokens', '128', '--requests', '1'], check=True, capture_output=True)
                (state / 'preferences.toml').write_text('mouse=false\nproject={}\nengine_path={}\n'.format(json.dumps(str(project)), json.dumps(str(a.engine.resolve()))))
                term = Terminal(a.alt.resolve(), state, project, width, height)
                offset = len(REQUESTS)
                def allowance(): term.send(b'\x0c')  # Ctrl+L, with the mouse disabled.
                def choose(index): term.send(b'\x1b[B' * index + b'\r')
                try:
                    term.wait('Alt'); term.send(b'\x0e'); term.wait('New conversation')
                    term.paste('First request'); term.send(b'\r'); term.wait('Waiting for model output'); term.wait('Receiving model output'); term.wait('Response ready'); term.wait('Calls left: 0')
                    assert len(REQUESTS) == offset + 1
                    term.paste('Continue the saved task'); term.send(b'\r'); term.wait('Model call allowance exhausted')
                    assert len(REQUESTS) == offset + 1, 'Exhaustion forwarded an extra model request'
                    # Close the recoverable error notice before using the labelled action.
                    term.send(b'\x1b'); allowance(); term.wait('Current connection allowance'); choose(0)
                    term.wait('Observed inference allowance'); term.wait('Model calls remaining: 0'); term.wait('Measured prompt: Unknown')
                    assert len(REQUESTS) == offset + 1, 'Inspecting allowance sent a model request'
                    term.send(b'\x1b'); allowance(); term.wait('Current connection allowance'); choose(1)
                    term.wait('Extra model calls'); term.send(b'\x15'); term.paste('2'); term.send(b'\r')
                    term.wait('Extra generated tokens'); term.send(b'\x15'); term.paste('256'); term.send(b'\r')
                    term.wait('Add this connection allowance?'); term.send(b'\t\r'); term.wait('Allowance added'); term.wait('Calls left: 2')
                    assert len(REQUESTS) == offset + 1, 'Allowance addition sent a prompt'
                    term.paste('Continue the original request'); term.send(b'\r'); term.wait('Response ready'); term.wait('Calls left: 1')
                    assert len(REQUESTS) == offset + 2
                    # Save allocation while retaining the session. Recover with explicit review.
                    allowance(); term.wait('Current connection allowance'); choose(2); term.wait('Model output and sampling'); choose(0)
                    term.wait('Set Total output tokens'); term.send(b'\x15'); term.paste('256'); term.send(b'\r'); term.wait('Model allocation saved')
                    allowance(); term.wait('Current connection allowance'); choose(3); term.wait('Reconnect with this allocation?'); term.send(b'\t\r'); term.wait('Ready. Describe')
                    assert len(REQUESTS) == offset + 2, 'Reconnect sent a prompt automatically'
                    with sqlite3.connect(state / 'sessions.db') as db:
                        payload = json.loads(db.execute('SELECT profile FROM sessions').fetchone()[0])
                        events = [json.loads(row[0]) for row in db.execute('SELECT payload FROM events')]
                    assert payload['model'] == MODEL and payload['inference']['output_tokens'] == 256
                    assert any(e.get('type') == 'allowance_added' and e['requests'] == 2 and e['generated_tokens'] == 256 for e in events)
                    assert any(e.get('type') == 'allocation_changed' and e['model_unchanged'] for e in events)
                    assert any(e.get('type') == 'user' and e['text'] == 'First request' for e in events)
                    term.close()
                    rows.append(dict(width=width, height=height, passed=True, actual_requests=2, addition_issued_no_request=True, allocation_persisted=True))
                    print('PASS: {}x{} observed stages, exhaustion, finite addition, continued task and saved-allocation reconnect'.format(width, height), flush=True)
                finally:
                    if term.proc.poll() is None:
                        term.proc.terminate(); term.proc.wait(timeout=10)
    finally:
        server.shutdown(); server.server_close()
    if a.output:
        a.output.parent.mkdir(parents=True, exist_ok=True)
        a.output.write_text(json.dumps(dict(schema=1, passed=True, binary_sha256=hashlib.sha256(a.alt.read_bytes()).hexdigest(), attempts=rows, scope='Actual PTY/Alt/Goose with a weight-free provider'), indent=2) + '\n')


if __name__ == '__main__':
    main()
