#!/usr/bin/env python3
"""Actual browser and optional scanner checks against disposable broken/fixed apps."""
import argparse,json,os,shutil,subprocess,tempfile,threading
from pathlib import Path
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--alt',type=Path,default=Path('target/debug/alt'));p.add_argument('--chromium',default='/usr/bin/chromium');p.add_argument('--semgrep',action='store_true');p.add_argument('--output',type=Path);a=p.parse_args();binary=a.alt.resolve();records=[]
class H(BaseHTTPRequestHandler):
 fixed=False
 def log_message(self,*args):pass
 def do_GET(self):
  body=("<button id='go' onclick=\"document.querySelector('#result').textContent='Ready'\">Run</button><div id='result'>Pending</div>" if self.fixed else "<button id='go'>Run</button><div id='result'>Pending</div>").encode();self.send_response(200);self.send_header('Content-Length',str(len(body)));self.end_headers();self.wfile.write(body)
server=ThreadingHTTPServer(('127.0.0.1',0),H);threading.Thread(target=server.serve_forever,daemon=True).start()
with tempfile.TemporaryDirectory(prefix='alt-pack-acceptance-') as t:
 root=Path(t);project=root/'project';project.mkdir();state=root/'state'
 def run(pack,inputs):
  r=subprocess.run([str(binary),'--data-dir',str(state),'--access','trusted','packs','run',pack,'--input',json.dumps(inputs)],cwd=project,capture_output=True,text=True,timeout=330)
  if not r.stdout.strip():raise AssertionError(r.stderr)
  record=json.loads(r.stdout);records.append(record);return r.returncode,record
 try:
  inputs={'url':f'http://127.0.0.1:{server.server_port}/','click':'#go','selector':'#result','contains':'Ready','executable':a.chromium}
  code,broken=run('browser',inputs);assert code!=0 and broken['status']=='findings',broken
  H.fixed=True;code,fixed=run('browser',inputs);assert code==0 and fixed['status']=='passed',fixed
  assert Path(fixed['input']['screenshot']).is_file()
  if a.output:
   a.output.parent.mkdir(parents=True,exist_ok=True)
   for record in records:
    source=Path(record['input']['screenshot'])
    if source.is_file():
     dest=a.output.parent/(record['id']+'.png');shutil.copy2(source,dest);record['retained_screenshot']=dest.name
  if a.semgrep:
   (project/'app.py').write_text('def calculate(value):\n    return eval(value)\n')
   code,broken=run('semgrep',{});assert code!=0 and any('dynamic-eval' in f['rule'] for f in broken['findings']),broken
   (project/'app.py').write_text('def calculate(value):\n    return int(value)\n')
   code,fixed=run('semgrep',{});assert code==0 and not fixed['findings'],fixed
  if a.output:
   a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps({'scope':'Actual browser / scanner; no inference model','records':records},indent=2)+'\n')
  print('PASS: browser detects broken click behavior, fixed UI passes and screenshot exists'+('; Semgrep detects seeded eval and fixed implementation passes' if a.semgrep else ''))
 finally:
  if a.output:
   a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps({"scope":"Actual browser/scanner; no inference model","records":records},indent=2)+"\n")
  server.shutdown()
