import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-atomic-save-2/independent/completion.json': '07fc00680c88758109b5c7f9a2b6557734f14e4d9211ca1a61e3db7a340a3960', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-2/independent/check.py': '11b9f042858b98fc80c4c53d9cc0cf2b32bb6880e88d070ea50d2eb6f6297c79'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_55f946a165a14ecea9f6f02884e616ed'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
