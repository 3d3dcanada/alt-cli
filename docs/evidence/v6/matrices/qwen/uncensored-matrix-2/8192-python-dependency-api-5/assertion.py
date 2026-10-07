import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-api-5/independent/completion.json': 'f5e55f102f9841c0c4cf8591c2c191c71398f0eab815570678e704e9d9159cfb', '/home/runner/work/_temp/campaign/8192-python-dependency-api-5/independent/check.py': '34049bb8ba4f44c3d9a1ce481f65d478aecb06c331836fe6937b23d93f45998c'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-api-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_971ea3202c514e719db577176181e905'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
