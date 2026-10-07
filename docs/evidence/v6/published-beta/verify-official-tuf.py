import functools, hashlib, http.server, json, os, pathlib, shutil, subprocess, threading

root = pathlib.Path('/workspace/.alt-evaluations/v6/published-beta')
source = root / 'trust-source'
mirror = root / 'local-tuf-mirror'
mirror.mkdir(exist_ok=True)
for path in (source / 'metadata/root_history').glob('*.json'):
    shutil.copyfile(path, mirror / path.name)
for path in (source / 'metadata').glob('*.json'):
    body = json.loads(path.read_text())
    shutil.copyfile(path, mirror / path.name)
    shutil.copyfile(path, mirror / f"{body['signed']['version']}.{path.name}")
targets = json.loads((source / 'metadata/targets.json').read_text())['signed']['targets']
(mirror / 'targets').mkdir(exist_ok=True)
for name, entry in targets.items():
    path = source / 'targets' / name
    body = path.read_bytes()
    if len(body) != entry['length']:
        raise ValueError('Target length mismatch')
    for algorithm, digest in entry['hashes'].items():
        if hashlib.new(algorithm, body).hexdigest() != digest:
            raise ValueError('Target hash mismatch')
        destination = mirror / 'targets' / f'{digest}.{name}'
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(body)

class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass

handler = functools.partial(Handler, directory=str(mirror))
server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler)
thread = threading.Thread(target=server.serve_forever, daemon=True)
thread.start()
env = os.environ.copy()
env['XDG_CACHE_HOME'] = '/workspace/.alt-tools/cache'
try:
    result = subprocess.run(['/workspace/.alt-tools/gh', 'attestation', 'trusted-root',
                             '--tuf-url', f'http://127.0.0.1:{server.server_port}/',
                             '--tuf-root', str(root / 'sigstore-embedded-root.json')],
                            env=env, capture_output=True, text=True)
finally:
    server.shutdown()
    server.server_close()
(root / 'official-metadata-validation.stderr').write_text(result.stderr)
if result.returncode:
    raise RuntimeError('TUF validation failed; inspect saved stderr')
(root / 'verified-trusted-root.jsonl').write_text(result.stdout)
metadata = json.loads((root / 'trust-bootstrap.json').read_text())
metadata.update({'transport': 'Git clone of official sigstore/root-signing, then loopback-only TUF mirror of unchanged signed bytes',
                 'source_commit': subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip(),
                 'tuf_signature_and_expiry_validation': True,
                 'trusted_root_sha256': hashlib.sha256(result.stdout.encode()).hexdigest(),
                 'reason': 'Both official CDN and Pages endpoints returned HTTP 403 in this cloud. GitHub Git transport was available. gh validated the signed TUF chain starting from the exact root bytes embedded in its official binary.'})
(root / 'trust-bootstrap.json').write_text(json.dumps(metadata, indent=2) + '\n')
print('Official Sigstore metadata passed TUF signature and expiry checks against the original embedded trust anchor.')
