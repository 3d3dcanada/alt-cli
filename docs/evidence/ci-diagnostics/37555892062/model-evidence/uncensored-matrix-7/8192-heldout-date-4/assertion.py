import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-date-4/independent/completion.json': '6da6f1216afd86471b3198e81a06b8c21ae79ce000578cad3081783b02c21965', '/home/runner/work/_temp/campaign/8192-heldout-date-4/independent/check.py': 'ed23f43a7c0297ad53cd7f6b8a4d4f01f9b0490cc4be6f223147e1ef27e0c590'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-date-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_14d32d6bd41541d8b70da1d96ff92551'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
