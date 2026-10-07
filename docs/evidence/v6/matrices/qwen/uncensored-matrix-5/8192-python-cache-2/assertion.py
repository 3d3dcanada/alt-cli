import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cache-2/independent/completion.json': 'c7cafe33f543769a2e781d385b5dba7cff8284b7a49204a9135b1c763aa8170e', '/home/runner/work/_temp/campaign/8192-python-cache-2/independent/check.py': '5f2e2a413a2b3a10b1cbbd33c424f09c09fc0fd8e60cde16bd9cf9d725ac2e9d'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cache-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_27154f351cc542e98e3c4ada2528474d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
