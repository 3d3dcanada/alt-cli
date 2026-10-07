import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-toposort-1/independent/completion.json': 'bfaea11be013c8453e58bbe02721ff06f713dec9540d66e8f130aac8b0b2c74a', '/home/runner/work/_temp/campaign/8192-heldout-toposort-1/independent/check.py': '3a1123413adef6092b43a17c912dac7a79602fb18698a5922b9fa789862ca344'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-toposort-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_6b7abead4d3c44b2820fcdb25c64d488'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
