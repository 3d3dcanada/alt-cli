import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-pagination-5/independent/completion.json': 'c353052f6fd29e27af9809ea2ef06cee28ad99774a404738ae82838f590f65d8', '/home/runner/work/_temp/campaign/8192-python-pagination-5/independent/check.py': '02f99410ee24ca51c0ac159013a7c0744cd4cf4c9777a8b4e5aad43928255ed9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-pagination-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_2333c0a8eae44818bc67cf0aae77807c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
