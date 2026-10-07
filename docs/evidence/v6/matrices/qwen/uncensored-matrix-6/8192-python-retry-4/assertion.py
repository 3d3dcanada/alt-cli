import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-retry-4/independent/completion.json': 'bdf231c75db36c9ed5083bdcdba235b8feaccabf12e3e905d4aabf5428b6cdf6', '/home/runner/work/_temp/campaign/8192-python-retry-4/independent/check.py': '7361723d7909bb9f5bdd6850c8ad1e446d05e3f09b3b6922067f19d23f059461'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-retry-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_3a895fd4203e4d3ebf8de117906e634c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
