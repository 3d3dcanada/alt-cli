import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-final-feature/8192-python-feature-1/independent/completion.json': '1e952947dafa4ad2b3361c4ff325858ee9b5aa2ab00b8e419773c3c39ae2c852', '/workspace/.alt-repair/mimo-final-feature/8192-python-feature-1/independent/check.py': '25c3e52bf6b3c68865e10dab4d2ca711f3887cf7ee313d9ecb094267d71c29c6'}
command=['python3', '/workspace/.alt-repair/mimo-final-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_8d98674e85d64d039e000f5b1dbd4c3d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
