import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-transaction-4/independent/completion.json': '1db74b34610624a0d6f176eb2277de796951d1714bcb3bd4f761ea74a77dae49', '/home/runner/work/_temp/campaign/8192-python-transaction-4/independent/check.py': 'cfcfd707adcd3083657715bf8af06182c0318bac13d2d5fcbb635d669226d29c'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-transaction-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_93ab6a8094db4cad89ae64804be7ac38'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
