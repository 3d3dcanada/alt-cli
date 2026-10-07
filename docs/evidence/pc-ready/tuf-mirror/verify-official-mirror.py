import hashlib,json,os,re,subprocess,threading,urllib.request
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path

root=Path('/workspace/.alt-pc-ready/tuf-mirror');root.mkdir(exist_ok=True)
commit=json.loads(subprocess.check_output(['/workspace/.alt-tools/gh','api','repos/sigstore/root-signing/commits/main']))['sha']
anchor=urllib.request.urlopen('https://raw.githubusercontent.com/sigstore/sigstore-go/v1.3.0/pkg/tuf/repository/root.json').read()
(root/'bootstrap-root.json').write_bytes(anchor)
receipts={}
class Mirror(BaseHTTPRequestHandler):
 def log_message(self,*args):pass
 def do_GET(self):
  try:
   name=self.path.lstrip('/')
   if re.fullmatch(r'\d+\.root.json',name):remote='metadata/root_history/'+name
   elif re.fullmatch(r'(?:\d+\.)?(?:root|timestamp|snapshot|targets)\.json',name):remote='metadata/'+re.sub(r'^\d+\.','',name)
   elif name.startswith('targets/'):
    target=name.removeprefix('targets/');target=re.sub(r'^[a-f0-9]{64}\.','',target)
    assert target=='trusted_root.json';remote='targets/'+target
   else:raise ValueError('Unexpected TUF path')
   data=urllib.request.urlopen('https://raw.githubusercontent.com/sigstore/root-signing/'+commit+'/'+remote,timeout=20).read()
   receipts[remote]=hashlib.sha256(data).hexdigest();(root/remote.replace('/','_')).write_bytes(data)
   self.send_response(200);self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
  except Exception:
   self.send_response(404);self.end_headers()
server=ThreadingHTTPServer(('127.0.0.1',0),Mirror)
threading.Thread(target=server.serve_forever,daemon=True).start()
env=dict(os.environ,XDG_CACHE_HOME='/workspace/.alt-tools/cache')
with (root/'trusted-root.jsonl').open('w') as out,(root/'verify.stderr').open('w') as err:
 result=subprocess.run(['/workspace/.alt-tools/gh','attestation','trusted-root','--tuf-url','http://127.0.0.1:'+str(server.server_port),'--tuf-root',str(root/'bootstrap-root.json')],stdout=out,stderr=err,env=env)
server.shutdown()
(root/'receipt.json').write_text(json.dumps(dict(source_repo='sigstore/root-signing',source_commit=commit,bootstrap_source='sigstore/sigstore-go v1.3.0 pkg/tuf/repository/root.json (CLI 2.102.0 dependency)',bootstrap_sha256=hashlib.sha256(anchor).hexdigest(),metadata=receipts,exit=result.returncode,scope='Official GitHub metadata mirror; gh verifies TUF signatures, expiries and target hashes before emitting the public Sigstore root. TLS and bundle verification remain enabled.'),indent=2)+'\n')
print('TUF mirror verification exit',result.returncode)
if result.returncode:print((root/'verify.stderr').read_text()[-1600:])
raise SystemExit(result.returncode)
