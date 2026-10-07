import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-deduplicate-3/independent/completion.json': 'f06508b769bc84f90660f5d7afcb0369b4426978c5dcd5d01c2f321e42042276', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-3/independent/check.py': 'c2221963eaa4ae83b9e19b6d20b3a1f558fd9f76beaaaa8991286711e42488d9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_09eee478fa0e4c9e80c4a3dfdf2e1492'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
