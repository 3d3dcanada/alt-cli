import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-atomic-save-4/independent/completion.json': '1c02a1d89243e3e4a70a6c2995e2c9764fc78bd428173009255a54a41b9c432d', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-4/independent/check.py': '6cb9cea4e17818c40a39211c841a9bc3bfd01a228b3ad8985ad1dbab9cd4539c'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_38c7ab20ded1485593c8829d87f88e28'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
