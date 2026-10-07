import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cli-3/independent/completion.json': '82005fcb04fc4342e0fbaf48f4f733a7ef3926e25e07566f183fbeebf07f2a43', '/home/runner/work/_temp/campaign/8192-python-cli-3/independent/check.py': '6caa9f6668b316e06f18816e5bd82765fd544bb8e04fc32e0cbe89517e517f08'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cli-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_41d32c5c73074658b78ef6e4dc1efd62'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
