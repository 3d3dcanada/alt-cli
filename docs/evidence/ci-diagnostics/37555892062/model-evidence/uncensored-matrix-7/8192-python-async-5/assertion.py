import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-async-5/independent/completion.json': '8252cf239b6716752fc6fc22027ffcb801d7a6f16306ed728d2f5c8163a37560', '/home/runner/work/_temp/campaign/8192-python-async-5/independent/check.py': '3428cfefa790110d7c6453f46d22225315473882f6efb07b6526f03a15e2f2dc'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-async-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_8c8b2138e020467eb04ee99da1a5c217'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
