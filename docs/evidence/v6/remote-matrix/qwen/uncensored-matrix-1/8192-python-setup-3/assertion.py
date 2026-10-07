import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-setup-3/independent/completion.json': 'bb1e58ff474c3a9eed3bc45916c6f096c83aad6008af1d3784a89899bdcbee48', '/home/runner/work/_temp/campaign/8192-python-setup-3/independent/check.py': 'fd350b54839552491fb98207aef1737cb118764fa16cf2d0db2e5a563ae76f66'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-setup-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_65065e054db44d429fa74ac6aca89a5e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
