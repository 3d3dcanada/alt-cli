import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-pagination-2/independent/completion.json': '3e2569956dca5ebe943bca767f11a66f35cb3f3c00c903f5682e09ed3b84bbc1', '/home/runner/work/_temp/campaign/8192-python-pagination-2/independent/check.py': '65cf4c0234cc39d0696ab3ec23fe31882c0d3063479fc2d7c03d54e9d4ea7bdd'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-pagination-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_7838dfda93724dd1a173bf5d49b8faaa'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
