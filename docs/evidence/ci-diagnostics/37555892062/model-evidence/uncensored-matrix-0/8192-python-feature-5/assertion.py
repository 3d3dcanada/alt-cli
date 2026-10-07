import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-feature-5/independent/completion.json': 'fab22d44d35c3ca4586947e32b9bbf7d050b158057184dfe694f8b5cdaec36ea', '/home/runner/work/_temp/campaign/8192-python-feature-5/independent/check.py': 'a0ed55a16960f1e4d5ccedcb185dd64ce8fae9ece1cdab1d85c140cc3f232e11'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-feature-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_f4ff8d64a10646f09098da5eb8ab5e65'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
