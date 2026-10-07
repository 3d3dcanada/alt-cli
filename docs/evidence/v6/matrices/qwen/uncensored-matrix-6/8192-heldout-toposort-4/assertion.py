import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-toposort-4/independent/completion.json': '99d0b832ad7884a35e7b2c5c2bee5984624b42806496c9f0bd4c319ff9e182f7', '/home/runner/work/_temp/campaign/8192-heldout-toposort-4/independent/check.py': 'de82bf16b0b9ed9aff3596497ca956dcf7a19a013a99d78503b6f8591c5e62d0'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-toposort-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_d93e8b645fb94e0f9f2d4b2f2d71e37b'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
