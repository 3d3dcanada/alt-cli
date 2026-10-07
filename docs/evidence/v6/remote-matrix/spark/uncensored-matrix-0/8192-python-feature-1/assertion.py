import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-feature-1/independent/completion.json': '4644e6620b87cda6c735d0250ffe38c1d4cb34dbd9b0e929ece377a8b3b14e2a', '/home/runner/work/_temp/campaign/8192-python-feature-1/independent/check.py': '05e024a7f7c1c2b4d6998a14ccf1f8db5ca99496f35cb15bb989fc8c6f0c2d91'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_726685f06a66449aabf01536c0e3e997'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
