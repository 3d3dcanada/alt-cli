import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-transaction-3/independent/completion.json': 'fdad3b861281c780497341aae4fb23d7ade28ab4bc9c4bb9016692654e28df6e', '/home/runner/work/_temp/campaign/8192-python-transaction-3/independent/check.py': 'ab18062a3a779549371f753bc57aaba2ed7db2e862fb320a2fc633d024ba2d34'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-transaction-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4ce448bb2d2b4a20a349e146f065d0b7'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
