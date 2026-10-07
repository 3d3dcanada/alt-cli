import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-toposort-2/independent/completion.json': 'db5e8fbb68de10a8d4bc57576959a854e87b7e1ac74a4e27843de6e96f677d8a', '/home/runner/work/_temp/campaign/8192-heldout-toposort-2/independent/check.py': '7ab99a4b4e3662660012c998bbbe6279f0e9001de7f03cb97bf6e9ac7c2a1ad1'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-toposort-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_5b0a134fee654c6e908106f65777ac3d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
