import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-migration-3/independent/completion.json': '51fe7b27019d9e369b12a943e2d737a72b6c8a66889952999f41b303a47e2bf4', '/home/runner/work/_temp/campaign/8192-python-migration-3/independent/check.py': '9963fc7cacaf67b2c99022cab35fe725c485f95607e41a83860bd5fb9e8e101f'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-migration-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_cadbd192357c4f61b1ecf0302daae46c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
