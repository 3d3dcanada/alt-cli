import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-setup-5/independent/completion.json': 'a320687872b823a3386c581522d4e4cac6cb7c3c57604d28bdb2ebee2df4bffe', '/home/runner/work/_temp/campaign/8192-python-setup-5/independent/check.py': '7fe6821578e64132416a8c8aff3f27cf6d380f9628846b0ff01b4c5d76b9f450'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-setup-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ebfed2947a1c4cf58853d74252c3aa5c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
