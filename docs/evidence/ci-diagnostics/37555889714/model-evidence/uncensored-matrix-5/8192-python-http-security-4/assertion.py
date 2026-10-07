import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-http-security-4/independent/completion.json': '757865515aaac0a38068e241ce61fe2d6bcbfcb6a0665bb74bafb557eafcc377', '/home/runner/work/_temp/campaign/8192-python-http-security-4/independent/check.py': 'df56b10465b731dcc3bf70bd2babd1d8e8e9ec98b7b7dbe57b01e9bfe72b2d98'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-http-security-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_426fa44c39e44ba1b4e9ee235238cfd3'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
