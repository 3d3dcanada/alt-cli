import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-setup-4/independent/completion.json': 'c8db9b2d2692a8536cb7381255628c06279dd015331246da3d91c052a56cada8', '/home/runner/work/_temp/campaign/8192-python-setup-4/independent/check.py': 'dc22d5d2e9a15149af45915da9ccb7e62a6b654103032457fc18e878c0bad6a3'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-setup-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_406fb54d5f154d65a1fd07518b3e3c52'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
