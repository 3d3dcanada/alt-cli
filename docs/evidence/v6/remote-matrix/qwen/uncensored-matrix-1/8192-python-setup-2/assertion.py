import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-setup-2/independent/completion.json': '3e3a9f22395fbef81c1909d5e3add17004cde715368c7e2c217fecf095e2e3d8', '/home/runner/work/_temp/campaign/8192-python-setup-2/independent/check.py': '347924bbf5324103302e2dbb29c9506cbe095c8c91c5103301e7e6ec509a8c9b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-setup-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_5e50e092dbcc4c88908ad433642ac694'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
