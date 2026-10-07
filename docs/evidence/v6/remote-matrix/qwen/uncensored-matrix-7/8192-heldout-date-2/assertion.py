import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-date-2/independent/completion.json': 'b726879abfc67d10102a275e6f44c425a4767905ed995dbc543bdf9d2727a621', '/home/runner/work/_temp/campaign/8192-heldout-date-2/independent/check.py': '2a584b63b79cce4ed99aa127e91c7d3b42a07e1ee63b56f275264b87b90f425b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-date-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_64b10e5de25c475398ab8d15765c49af'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
