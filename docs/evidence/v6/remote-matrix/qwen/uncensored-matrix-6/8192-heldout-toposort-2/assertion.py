import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-toposort-2/independent/completion.json': 'b8539aa8f9acb3ea56f58fd29e4f04495858707e3f008368312026368ce7b80b', '/home/runner/work/_temp/campaign/8192-heldout-toposort-2/independent/check.py': 'e8f80509eb40bd1e8dc0c421182eab33db0c8b08b3140ee815e4961d62d57132'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-toposort-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c090684fb8ef429695fc01ee758ac0fa'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
