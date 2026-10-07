import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-async-1/independent/completion.json': '6d5abe58a99dd0cb57f1bffcf67ae0945ced0a364f2f4b545549fd8de245d833', '/home/runner/work/_temp/campaign/8192-python-async-1/independent/check.py': 'ea7ea1d20842c43feac72982a73100798063f10c959c5ca6af731bb0dd6e7651'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-async-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_f2bd24c5d1fd4ce08098b65d98e1cfb2'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
