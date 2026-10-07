import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-multifile-3/independent/completion.json': '1de88269cc2764ddb3350f1d88f334c5be6bf9fcb7bd4ea5148d5e23bbfa79e3', '/home/runner/work/_temp/campaign/8192-python-multifile-3/independent/check.py': 'aef334dd351333475e37f7e9ae53c734840e8f4271b24365d946e64a318f36a5'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-multifile-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_cb9f5b2af21f42ba8104b36171dd75c0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
