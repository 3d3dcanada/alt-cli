import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-toposort-3/independent/completion.json': 'faa4dea6be6b5278cc8f075ce417aeac304e9de730ec03a50b190e3aace1b0f5', '/home/runner/work/_temp/campaign/8192-heldout-toposort-3/independent/check.py': '3eed334fb97e36698253e3329c497a627d801cc0e071156fa9ef84f1a33a88b1'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-toposort-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4e39f3c9518148d898bf626cb3bc2f9f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
