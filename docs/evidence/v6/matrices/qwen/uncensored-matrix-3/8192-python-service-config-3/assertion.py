import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-service-config-3/independent/completion.json': '67f795a25d462e5618bf4ecc98f14e9fb369ba6893dd81c9cbfe0c7362fc26f7', '/home/runner/work/_temp/campaign/8192-python-service-config-3/independent/check.py': '9fe3d4cb8c7f5158c8da55c503416d33e8ac6c2d4491bb77a4b935a7dfd60271'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-service-config-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_f549534600bc463383d39ca0f2c7f3fb'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
