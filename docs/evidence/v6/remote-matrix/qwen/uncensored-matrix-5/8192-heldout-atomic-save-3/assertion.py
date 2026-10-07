import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-atomic-save-3/independent/completion.json': '62b2185597763029646e72310ca5a23c66fb7fd9f480237ae4377ab8ad51fe2a', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-3/independent/check.py': '3ea29df6c2e53dbda7bd94eee4d51ecd990f1cfe76bb212327d7d4ef5c495b3b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_dcadc2667e4e4651a296cd4c9c560317'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
