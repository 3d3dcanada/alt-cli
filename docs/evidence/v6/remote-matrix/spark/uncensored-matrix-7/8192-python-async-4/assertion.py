import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-async-4/independent/completion.json': '56bdef9ebd5b7a2344c98b71c47407c2b88dfb0ca4690d7e02a86ca071a3090e', '/home/runner/work/_temp/campaign/8192-python-async-4/independent/check.py': '493dd32f7cad4847ad144612b592413f3077d63adaadd747459b36050915c554'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-async-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4f790d77216449f384d5d0a099111cdd'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
