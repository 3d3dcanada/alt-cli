import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-jsonl-5/independent/completion.json': '9ef898e9c48e6d55ddb8b7c18080badfcfc435e0ceb5fc6d9fdec6aaf014d0cf', '/home/runner/work/_temp/campaign/8192-python-jsonl-5/independent/check.py': '1b41375b51a900d6fe6033fb4334333c2f81c62dde6da08fdc2ca4dcf58ad12b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-jsonl-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_0c57e68db89e4d6eab8a029449cb5cb2'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
