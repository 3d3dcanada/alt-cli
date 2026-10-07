import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-feature-5/independent/completion.json': '25f2994099692cdb1cb67c079efcc93b65d99abfe5a3c899d2e59c4739822ae1', '/home/runner/work/_temp/campaign/8192-python-feature-5/independent/check.py': '15f2d75559df195973a68f1a0c94624b75095e62d69089c33e9c186d0ad5ad11'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-feature-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a8c856cc7f0d4b24b69d039be526fd05'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
