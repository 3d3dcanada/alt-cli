import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-atomic-save-3/independent/completion.json': '942db342b0637f7ad35ed8f331c67f743a7593bf67083cc6e72bbc8524ea3e81', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-3/independent/check.py': '967b362ac92b0721430c37437e91884cd3ad771d79fa67a1f8158e49df5c4ed0'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_147a3c00da434ea7bbc70f594103f800'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
