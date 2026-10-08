# Runtime audit reproductions

Observed against `/workspace/alt-cli/target/debug/alt` at repository commit
`fa0c8e4130ec4d13c59193436679c4e059c22dee` (beta 3). These checks used disposable
state, a deliberately invalid four-byte GGUF fixture, an executable shell marker,
and a loopback HTTP fixture. No actual model weights or inference were used.

Retain only this README and these two result receipts in the audit report:

- `audit-summary.json`
- `ollama-auth-reproduction.json`

The paths in those receipts identify the original disposable run; they are not
installation instructions. Do not copy the generated state directory or fixtures
into the report. A setup prototype initially placed `runtime_path` in
`runtime.toml` and was rejected as a schema error. The recorded runtime-fallback
result below uses the correct `preferences.toml` field.

## Cases 1 and 2: incomplete split import and explicit runtime fallback

Run from the checkout. This is the equivalent consolidated command for the
successful original reproduction; it creates a new disposable directory.

```bash
python3 - <<'PY'
from pathlib import Path
import json, subprocess, tempfile

alt = Path('target/debug/alt').resolve()
root = Path(tempfile.mkdtemp(prefix='alt-runtime-audit-', dir='/tmp'))
state = root / 'state'
first = root / 'fixture-00001-of-00002.gguf'
first.write_bytes(b'GGUF')
p = subprocess.run(
    [str(alt), '--data-dir', str(state), 'models', 'import', str(first),
     '--uncensored', '--use'], text=True, capture_output=True, timeout=15)
(root / 'import.stdout').write_text(p.stdout)
(root / 'import.stderr').write_text(p.stderr)
artifact = json.loads(p.stdout)

chosen = root / 'chosen-cuda-runtime-missing'
(state / 'preferences.toml').write_text(
    'runtime_path = ' + json.dumps(str(chosen)) + '\n')
fallback = state / 'tools/llama-b11429/llama-server'
fallback.parent.mkdir(parents=True)
fallback.write_text(
    '#!/bin/sh\necho AUDIT_UNREQUESTED_FALLBACK_USED >&2\nexit 127\n')
fallback.chmod(0o700)
b = subprocess.run([str(alt), '--data-dir', str(state), 'benchmark'],
                   text=True, capture_output=True, timeout=30)
(root / 'benchmark.stdout').write_text(b.stdout)
(root / 'benchmark.stderr').write_text(b.stderr)
report = json.loads(b.stdout)
result = {
    'evidence_dir': str(root),
    'four_byte_GGUF_accepted': artifact['bytes'] == 4,
    'split_second_file_exists': (root / 'fixture-00002-of-00002.gguf').exists(),
    'registered_pieces': artifact['pieces'],
    'explicit_selected_runtime': str(chosen),
    'fallback_used': 'AUDIT_UNREQUESTED_FALLBACK_USED' in report.get('error', ''),
    'benchmark_error': report.get('error'),
}
(root / 'audit-summary.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
PY
```

Observed: the invalid import was accepted with four bytes and no recorded parts;
the second shard was absent. Alt then launched the managed shell marker despite
the explicit missing runtime selection. The shell marker immediately exited 127;
the benchmark retained that failure. A corrected build should reject the import
and should never invoke a fallback for a broken explicit runtime choice.

## Case 3: authenticated Ollama inventory versus doctor

The original environment had a discoverable Goose executable. `doctor` rejected
Ollama authentication before spawning it. This command uses a literal dummy token
with no access to any service; the HTTP server binds only to loopback.

```bash
python3 - <<'PY'
from pathlib import Path
import http.server, json, os, subprocess, tempfile, threading

alt = Path('target/debug/alt').resolve()
root = Path(tempfile.mkdtemp(prefix='alt-ollama-auth-audit-', dir='/tmp'))
state = root / 'state'
seen = []
class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        seen.append({'path': self.path, 'dummy_auth_received':
                     self.headers.get('Authorization') == 'Bearer audit-dummy-not-a-secret'})
        data = json.dumps({'models': [{'name': 'audit-fixture-no-weights'}]}).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)
    def log_message(self, *args):
        pass
server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
env = dict(os.environ, AUDIT_DUMMY_TOKEN='audit-dummy-not-a-secret')
def invoke(*args):
    return subprocess.run([str(alt), '--data-dir', str(state), *args],
                          env=env, text=True, capture_output=True, timeout=10)
try:
    init = invoke('init', '--model', 'audit-fixture-no-weights', '--provider', 'ollama',
                  '--endpoint', f'http://127.0.0.1:{server.server_port}',
                  '--api-key-env', 'AUDIT_DUMMY_TOKEN')
    listing = invoke('models')
    doctor = invoke('doctor')
finally:
    server.shutdown()
    server.server_close()
out = {'init_status': init.returncode, 'inventory_status': listing.returncode,
       'inventory': listing.stdout.strip(), 'doctor_status': doctor.returncode,
       'doctor_error': doctor.stderr.strip(), 'requests': seen}
(root / 'ollama-auth-reproduction.json').write_text(json.dumps(out, indent=2) + '\n')
print(json.dumps(out, indent=2))
PY
```

Observed: `init` and `models` returned zero, both inventory requests carried the
dummy bearer value, and `doctor` returned one with:
`alt: Ollama authentication is not implemented in this slice`.

These commands reproduce the beta 3 findings. They are not application regression
tests and were not rerun while preparing this README. The original receipts remain
unchanged. A future fixed build may intentionally stop matching these observations.
