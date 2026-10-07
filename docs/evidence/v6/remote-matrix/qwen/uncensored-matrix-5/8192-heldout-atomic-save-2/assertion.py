import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-atomic-save-2/independent/completion.json': 'ece2e980d86712a987629c28dc8db14738e08aaed8f0ddf1f6be58e46174877a', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-2/independent/check.py': '968931db9681da66a8589cc6ffa40600076305c09879b60a4f42ad4eac70ab6b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_686c95779f644053a40836cff1e972ef'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
