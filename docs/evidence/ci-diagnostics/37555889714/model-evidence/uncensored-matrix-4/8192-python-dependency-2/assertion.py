import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-2/independent/completion.json': '141507dc2182ca4ecdad9ae91154bee56188dee0e7ef78b1302e75a79522130b', '/home/runner/work/_temp/campaign/8192-python-dependency-2/independent/check.py': '732ee0d890abb69413a0dbe22da074f8677b9ec21859769919006cef7a845458'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_57ebc357fd4f4c11a80c17cc36f48f6f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
