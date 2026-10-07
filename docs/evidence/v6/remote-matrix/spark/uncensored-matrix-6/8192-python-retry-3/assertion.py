import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-retry-3/independent/completion.json': '03c4fc9fa9abf2b3f132f57dff3a82f850db5c236f85cd2a9ef3016a7653257d', '/home/runner/work/_temp/campaign/8192-python-retry-3/independent/check.py': 'c3d4aec026f6741a8dd88dd955e04ac5b081bf0ebbfb0ae32dc953e06dbd3f4b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-retry-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_85a6e4a08ae042db8f78d39c561d115f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
