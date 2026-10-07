import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-pagination-4/independent/completion.json': 'a7f6648cc1c4e6b5280b70a02a17431b8199f265ce8d57915b8a0f4799591a0a', '/home/runner/work/_temp/campaign/8192-python-pagination-4/independent/check.py': '44d168a7071a6710f59383d0e9b655905e9f343ab5f7a4a66b25848f415acb46'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-pagination-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_5550f6ad92c0407492adbce9fad19de0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
