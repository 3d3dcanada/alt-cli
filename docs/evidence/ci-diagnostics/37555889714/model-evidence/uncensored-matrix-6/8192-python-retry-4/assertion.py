import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-retry-4/independent/completion.json': '9e358c11f322a9d8b6926e287c25ff99ab675c123e84057e69e898dddb8d3b25', '/home/runner/work/_temp/campaign/8192-python-retry-4/independent/check.py': '714a5854ebd36b59682503359036620d3251830d15cb1731d955baa358d318d9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-retry-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_cfc21b167f8c4c3d9215fa6ed736994c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
