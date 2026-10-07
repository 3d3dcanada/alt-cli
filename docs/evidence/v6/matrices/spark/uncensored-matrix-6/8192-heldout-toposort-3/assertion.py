import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-toposort-3/independent/completion.json': 'ac7f05c6f5771dd4c1c891e2e3132c5d77b154ecb349df19062fb116763aa609', '/home/runner/work/_temp/campaign/8192-heldout-toposort-3/independent/check.py': '27b04cea603b89f4ae3576dabbced044b04822535e5d793d5aaed097692cb96e'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-toposort-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ce4291eee62341728fb22e86c8c86f0f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
