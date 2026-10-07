import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-setup-2/independent/completion.json': '437cf49dbcea2fcea81a93a18a67b8262c2d49f6330861ab726847f4be687815', '/home/runner/work/_temp/campaign/8192-python-setup-2/independent/check.py': '7fe90016c7bd71310d14ee4be7ddcf4eca5d37312508d9e7e36bfa16e12aa0c2'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-setup-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4a05387121784f26b4d2d251b5f65a9c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
