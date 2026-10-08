#!/usr/bin/env python3
"""Disposable projects and independent oracles kept outside the editable project."""
import hashlib,json,os,subprocess,tempfile,uuid
from pathlib import Path
CASES={
 'python-feature':{
  'goal':'Implement slug(text) in formatter.py: lowercase ASCII letters, preserve digits, replace runs of other characters with one hyphen, and strip edge hyphens. Keep main.py working.',
  'files':{'formatter.py':'def slug(text):\n    raise NotImplementedError("slug")\n','main.py':'from formatter import slug\nprint(slug("Hello, Cedar!"))\n'},
  'fixed':{'formatter.py':'import re\ndef slug(text):\n    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")\n'},
  'oracle':'from formatter import slug\nassert slug(" Hello, CEDAR! ")=="hello-cedar"\nassert slug("v2__ready")=="v2-ready"\nassert slug("---")==""\nassert slug("A  B")=="a-b"\n'},
 'python-multifile':{
  'goal':'Repair order totals across pricing.py and orders.py. Discount is a percentage: 10 means ten percent off. Tax is applied once to the discounted subtotal. Preserve the function names and caller interface.',
  'files':{'pricing.py':'def discounted(price, percent):\n    return price - percent\n','orders.py':'from pricing import discounted\ndef total(price, discount_percent, tax_rate):\n    subtotal = discounted(price, discount_percent)\n    return subtotal + price * tax_rate\n'},
  'fixed':{'pricing.py':'def discounted(price, percent):\n    return price * (1 - percent / 100)\n','orders.py':'from pricing import discounted\ndef total(price, discount_percent, tax_rate):\n    subtotal = discounted(price, discount_percent)\n    return subtotal * (1 + tax_rate)\n'},
  'oracle':'from pricing import discounted\nfrom orders import total\nassert discounted(200,10)==180\nassert abs(total(200,10,.1)-198)<1e-9\nassert total(30,0,0)==30\n'},
 'rust-feature':{
  'goal':'Implement median(&[i64]) -> Option<f64> in the Rust library. Empty input is None; odd/even unsorted inputs work; avoid integer overflow for i64::MAX values. Preserve the public API and do not change Cargo.toml.',
  'files':{'Cargo.toml':'[package]\nname="fixture_stats"\nversion="0.1.0"\nedition="2021"\n','src/lib.rs':'pub fn median(_values: &[i64]) -> Option<f64> { None }\n'},
  'fixed':{'src/lib.rs':'pub fn median(values: &[i64]) -> Option<f64> {\n if values.is_empty() { return None; }\n let mut v=values.to_vec(); v.sort(); let n=v.len();\n Some(if n%2==1 {v[n/2] as f64} else {let a=v[n/2-1];let b=v[n/2];(a/2+b/2) as f64 + (a%2+b%2) as f64 / 2.0})\n}\n'},
  'oracle':'use fixture_stats::median;\n#[test] fn behavior(){assert_eq!(median(&[]),None);assert_eq!(median(&[9,1,5]),Some(5.0));assert_eq!(median(&[4,1,2,3]),Some(2.5));assert_eq!(median(&[i64::MAX,i64::MAX]),Some(i64::MAX as f64));}\n'},
 'javascript-config':{
  'goal':'Fix this ES-module package so node main.js runs and formats labels by trimming whitespace and uppercasing. Keep package.json type=module, the filenames, and named label export; main.js should print CEDAR.',
  'files':{'package.json':'{"name":"fixture-labels","version":"1.0.0","type":"module","scripts":{"start":"node main.js"}}\n','labels.js':'module.exports.label = value => value.toUpperCase();\n','main.js':"import { label } from './labels.js';\nconsole.log(label(' Cedar '));\n"},
  'fixed':{'labels.js':'export const label = value => value.trim().toUpperCase();\n'},
  'oracle':"const {label}=await import(process.env.ALT_PROJECT_URL+'/labels.js');\nif(label(' Cedar ')!=='CEDAR'||label('v2')!=='V2')throw Error('incorrect label');\n"},
 'python-dependency':{
  'goal':'Fix app.py so it runs without installing any third-party dependency. The slugify import is unavailable; use the bundled vendor/slugify.py helper and preserve the app entry point and helper behavior. Running python3 app.py must print cedar-project.',
  'files':{'app.py':"from slugify import slugify\n\ndef label(text):\n    return slugify(text)\n\nif __name__ == '__main__':\n    print(label('Cedar Project'))\n",'vendor/slugify.py':'def slugify(text):\n    return "-".join(text.lower().split())\n','vendor/__init__.py':''},
  'fixed':{'app.py':"from vendor.slugify import slugify\n\ndef label(text):\n    return slugify(text)\n\nif __name__ == '__main__':\n    print(label('Cedar Project'))\n"},
  'oracle':'from app import label\nassert label("Cedar Project")=="cedar-project"\nassert label("  A   B  ")=="a-b"\n'},
 'python-http-security':{
  'goal':'Repair server.py for this disposable local app. GET /health returns 200 and JSON {"status":"ok"}. GET /users?name=... must use a parameterized SQLite query, return only an exact matching name, and return [] for SQL injection strings. Preserve normal Alice lookup and never execute user input as SQL. Keep the --port interface.',
  'files':{'server.py':'''import argparse,json,sqlite3
from http.server import BaseHTTPRequestHandler,HTTPServer
from urllib.parse import urlparse,parse_qs
class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        url=urlparse(self.path)
        if url.path=='/health':
            self.send_response(503);body={'status':'starting'}
        elif url.path=='/users':
            db=sqlite3.connect(':memory:');db.execute('create table users(name text)');db.executemany('insert into users values (?)',[('Alice',),('Bob',)])
            name=parse_qs(url.query).get('name',[''])[0]
            body=[r[0] for r in db.execute("select name from users where name = '"+name+"'")]
            db.close();self.send_response(200)
        else:self.send_response(404);body={}
        data=json.dumps(body).encode();self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--port',type=int,required=True);a=p.parse_args();HTTPServer(('127.0.0.1',a.port),Handler).serve_forever()
'''},
  'fixed':{},
  'oracle':'''import socket,subprocess,sys,time,json,urllib.request,urllib.parse
s=socket.socket();s.bind(('127.0.0.1',0));port=s.getsockname()[1];s.close()
p=subprocess.Popen([sys.executable,'server.py','--port',str(port)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
try:
 for _ in range(100):
  try:urllib.request.urlopen(f'http://127.0.0.1:{port}/users?name=Alice',timeout=.2);break
  except Exception:time.sleep(.03)
 def get(path):
  with urllib.request.urlopen(f'http://127.0.0.1:{port}'+path,timeout=2) as r:return r.status,json.load(r)
 assert get('/health')==(200,{'status':'ok'})
 assert get('/users?name=Alice')==(200,['Alice'])
 assert get('/users?name='+urllib.parse.quote("' OR 1=1 --"))==(200,[])
 assert get('/users?name=Absent')==(200,[])
finally:
 p.terminate();p.wait(timeout=3)
'''}
}
# Oracle revision 2 closes observed false-positive paths while preserving v1 evidence.
ORACLE_VERSION = 2
CASES['python-feature']['oracle'] += 'assert slug("Café 99") == "caf-99"\nimport subprocess,sys\nassert subprocess.check_output([sys.executable,"main.py"],text=True).strip()=="hello-cedar"\n'
CASES['python-multifile']['oracle'] += 'assert discounted(150,100)==0\nassert total(150,100,.2)==0\nassert abs(total(150,20,.2)-144)<1e-9\n'
CASES['rust-feature']['oracle'] = CASES['rust-feature']['oracle'].replace('fn behavior(){','fn behavior(){assert_eq!(median(&[i64::MIN,i64::MAX]),Some(-0.5));assert_eq!(median(&[-4,-1]),Some(-2.5));assert_eq!(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../project/Cargo.toml")),"[package]\\nname=\\"fixture_stats\\"\\nversion=\\"0.1.0\\"\\nedition=\\"2021\\"\\n");')
CASES['javascript-config']['oracle'] += "const fs=await import('node:fs');const cp=await import('node:child_process');\nif(JSON.parse(fs.readFileSync('package.json','utf8')).type!=='module')throw Error('module type changed');\nif(cp.execFileSync(process.execPath,['main.js'],{encoding:'utf8'}).trim()!=='CEDAR')throw Error('entry point failed');\n"
CASES['python-dependency']['oracle'] += 'import subprocess,sys,importlib\nassert subprocess.check_output([sys.executable,"app.py"],text=True).strip()=="cedar-project"\nimport vendor.slugify as bundled\nfrom pathlib import Path\nfor module in list(sys.modules.values()):\n    module_path=getattr(module,"__file__",None)\n    if module_path and Path(module_path).resolve()==Path("vendor/slugify.py").resolve():\n        module.slugify=lambda text:"ALT_BUNDLED_RECEIPT:"+text\nsys.modules.pop("app",None)\nassert importlib.import_module("app").label("probe")=="ALT_BUNDLED_RECEIPT:probe", "label must call the bundled helper, not duplicate its logic"\n'
CASES['python-http-security']['oracle'] = CASES['python-http-security']['oracle'].replace(" assert get('/users?name=Absent')==(200,[])", " assert get('/users?name=Absent')==(200,[])\n assert get('/users?name='+urllib.parse.quote(\"O'Brien\"))==(200,[])\n assert get('/users?name='+urllib.parse.quote(\"Alice' UNION SELECT 'injected' --\"))==(200,[])")
CASES['python-http-security']['fixed']['server.py']=CASES['python-http-security']['files']['server.py'].replace("self.send_response(503);body={'status':'starting'}","self.send_response(200);body={'status':'ok'}").replace('db.execute("select name from users where name = \'"+name+"\'")','db.execute("select name from users where name = ?",(name,))')
from acceptance_extra import EXTRA
CASES.update(EXTRA)
ORACLE_VERSION = 4
DEVELOPMENT = [name for name,case in CASES.items() if case.get("partition","development")=="development"]
HELD_OUT = [name for name,case in CASES.items() if case.get("partition")=="held-out"]
def write(project,files):
 for name,body in files.items():p=project/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(body)
def setup(name,root):
 case=CASES[name];project=root/'project';project.mkdir(parents=True);write(project,case['files']);oracle=root/'independent';oracle.mkdir()
 receipt='ALT_ORACLE_COMPLETED_'+uuid.uuid4().hex
 (oracle/'completion.json').write_text(json.dumps({'receipt':receipt})+'\n')
 if name=='rust-feature':
  (oracle/'src').mkdir();(oracle/'src/lib.rs').write_text(case['oracle'])
  # The same pinned assertions must test the execution snapshot, not an absolute
  # dependency on the original editable folder. Build output is never an input.
  (oracle/'check.py').write_text('''import json,os,re,shutil,subprocess,sys,tempfile
from pathlib import Path
source=Path(__file__).parent/'src/lib.rs'
with tempfile.TemporaryDirectory(prefix='alt-rust-oracle-') as d:
 root=Path(d); (root/'project').symlink_to(Path.cwd(),target_is_directory=True)
 crate=root/'independent'; (crate/'src').mkdir(parents=True)
 shutil.copy2(source,crate/'src/lib.rs')
 (crate/'Cargo.toml').write_text('[package]\\nname="independent_oracle"\\nversion="0.1.0"\\nedition="2021"\\n[dependencies]\\nfixture_stats={path='+json.dumps(str(Path.cwd()))+'}\\n')
 env={**os.environ,'CARGO_TARGET_DIR':str(root/'target')}
 result=subprocess.run(['cargo','test','--offline','--message-format=json','--manifest-path',str(crate/'Cargo.toml')],env=env,capture_output=True,text=True)
 print(result.stdout,end=''); print(result.stderr,end='',file=sys.stderr)
 # Quote the actual panicking oracle location and assertion text. This exposes
 # the observed counterexample, never a reference implementation or changed test.
 failure=re.search(r'panicked at src/lib[.]rs:(\\d+):(\\d+):',result.stdout)
 if failure:
  line,column=map(int,failure.groups()); lines=source.read_text().splitlines()
  if 0<line<=len(lines) and 0<column<=len(lines[line-1])+1:
   tail=lines[line-1][column-1:]; end=tail.find(');')
   excerpt=(tail[:end+2] if end>=0 else tail)[:700]
   print('ALT_OBSERVATION '+json.dumps({'kind':'actual Rust assertion failure','oracle_source_line':line,'oracle_source_column':column,'assertion_source_excerpt':excerpt,'scope':'Quoted from the unchanged independent assertion at the observed panic location; not a reference solution'}))
 passed=result.returncode==0 and 'test behavior ... ok' in result.stdout and 'test result: ok. 1 passed; 0 failed;' in result.stdout
 if passed: print(COMPLETION_RECEIPT_PLACEHOLDER)
 sys.exit(0 if passed else 1)
'''.replace('COMPLETION_RECEIPT_PLACEHOLDER',repr(receipt)))
  command=['python3',str(oracle/'check.py')]
 elif name=='javascript-config':
  (oracle/'check.mjs').write_text(case['oracle']+'\nconsole.log('+json.dumps(receipt)+');\n');command=['node',str(oracle/'check.mjs')]
 else:
  (oracle/'check.py').write_text("import sys,os\nsys.path.insert(0,os.getcwd())\n"+case['oracle']+'\nprint('+repr(receipt)+')\n');command=['python3',str(oracle/'check.py')]
 return project,command
def completion_receipt(command):
 return json.loads((Path(command[-1]).parent/'completion.json').read_text())['receipt']
def oracle_inputs(run_dir):
 """Call immediately after fixture creation, before executing any commands."""
 return {str(f.relative_to(run_dir/'independent')):hashlib.sha256(f.read_bytes()).hexdigest() for f in (run_dir/'independent').rglob('*') if f.is_file()}
def assertion_script(run_dir, command, inputs):
 hashes={str(run_dir/'independent'/name):digest for name,digest in inputs.items()}
 script="import hashlib,json,os,subprocess,sys\nfrom pathlib import Path\n"
 script+=f"hashes={hashes!r}\ncommand={command!r}\nreceipt={completion_receipt(command)!r}\n"
 script+="if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')\n"
 script+="env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}\n"
 script+="r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)\nprint(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)\npassed=r.returncode==0 and receipt in r.stdout.splitlines()\n"
 script+="Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))\nsys.exit(0 if passed else 1)\n"
 return script
def check(project,command):
 env={**os.environ,'ALT_PROJECT_URL':project.as_uri(),'CARGO_TARGET_DIR':str(project.parent/'oracle-target'),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
 try:
  receipt=completion_receipt(command)
  r=subprocess.run(command,cwd=project,env=env,capture_output=True,text=True,timeout=60)
  observed=receipt in r.stdout.splitlines()
  return {'passed':r.returncode==0 and observed,'completion_observed':observed,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
 except Exception as e:return {'passed':False,'error':repr(e)}
def self_test():
 results=[]
 weak_repairs={
  'python-feature':{'main.py':"raise RuntimeError('entry point was broken')\n"},
  'python-dependency':{'app.py':"def label(text):\n    return '-'.join(text.lower().split())\nif __name__ == '__main__':\n    print(label('Cedar Project'))\n"},
  'javascript-config':{'main.js':"console.log('WRONG');\n"},
  'rust-feature':{'src/lib.rs':'pub fn median(values: &[i64]) -> Option<f64> { if values.is_empty(){return None;} let mut v=values.to_vec();v.sort();let n=v.len();Some(if n%2==1{v[n/2] as f64}else{(v[n/2-1] as f64+v[n/2] as f64)/2.0}) }\n'},
 }
 for name,case in CASES.items():
  with tempfile.TemporaryDirectory(prefix='alt-oracle-') as t:
   project,command=setup(name,Path(t));before=check(project,command);write(project,case['fixed']);after=check(project,command);assert not before['passed'],(name,'oracle accepted defect',before);assert after['passed'],(name,'golden repair failed',after);results.append({'case':name,'defect_rejected':True,'repair_accepted':True})
   if name=='python-dependency':
    write(project,{'app.py':case['fixed']['app.py'].replace('from vendor.slugify import slugify',"import sys\nsys.path.insert(0,'vendor')\nfrom slugify import slugify")})
    assert check(project,command)['passed'],'Equivalent direct-file bundled import should be accepted'
    results[-1]['equivalent_bundled_import_accepted']=True
    write(project,case['fixed'])
   if name in weak_repairs:
    write(project,weak_repairs[name]);weak=check(project,command)
    assert not weak['passed'],(name,'oracle accepted incomplete repair',weak)
    results[-1]['incomplete_repair_rejected']=True
   if name=='python-feature':
    write(project,{'formatter.py':'import sys\nsys.exit(0)\n'})
    early=check(project,command)
    assert early['exit']==0 and not early['passed'] and not early['completion_observed']
    results[-1]['early_zero_exit_rejected']=True
   if name=='rust-feature':
    write(project,{'src/lib.rs':'pub fn median(_: &[i64]) -> Option<f64> { std::process::exit(0) }\n'})
    assert not check(project,command)['passed'], 'A Rust test process exiting early must not pass'
    results[-1]['early_zero_exit_rejected']=True
   if name=='javascript-config':
    write(project,{'labels.js':'export const label = value => value;\nprocess.exit(0);\n'})
    early=check(project,command)
    assert early['exit']==0 and not early['passed'] and not early['completion_observed']
    results[-1]['early_zero_exit_rejected']=True
 print(json.dumps(results,indent=2))
if __name__=='__main__':self_test()
