import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-deduplicate-2/independent/completion.json': '02d3dd4617beb5d4f296cf8d2129dc65af9bcc4ba6ae035102a8151f3fccf6eb', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-2/independent/check.py': '3d19006559dbbb24432668f9ebdea7fbf0e70f88a80b11321977987f3a1d01ce'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_72eb6f4c3c394ec6a21cadbfe8d8650b'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
