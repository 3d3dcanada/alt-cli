import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-feature-3/independent/completion.json': '3c5f37f0166a2ea83a98ef6da761a2405a3d30bb7c7167bf5b0981b28e1f8936', '/home/runner/work/_temp/campaign/8192-python-feature-3/independent/check.py': '02605e191d842f1a75318a5fb6ce7a3358b2d3e43d0e7aed391ac84464214511'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-feature-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_74e14ce35d354bb3b2ecc996babfbabb'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
