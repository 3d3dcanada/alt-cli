import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-api-4/independent/completion.json': '9b9fb0823fbfc8ea034d26aeec70b1e1079c2b0f0f99604625f87199a1c2e74d', '/home/runner/work/_temp/campaign/8192-python-dependency-api-4/independent/check.py': '4199c1c2407f64fc0ce5e03f9cba5eef91f8b75d320572b046aa15f7216688d5'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-api-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_dfa5f56a36c94d5398225ab07ffc5d8e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
