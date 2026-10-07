import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-config-1/independent/completion.json': '25bcddf2dc6f0d7cc8b88e52ad6f32624032c3d707a8fc8f6b939d3f6e85ca84', '/home/runner/work/_temp/campaign/8192-python-config-1/independent/check.py': '889f90ea60548f0b78b94835a6d6f5dbc9ec68439d30adcd72a2a1efa76f35c9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-config-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_fde0e1b745dc4fb2a3c9029a09ad758f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
