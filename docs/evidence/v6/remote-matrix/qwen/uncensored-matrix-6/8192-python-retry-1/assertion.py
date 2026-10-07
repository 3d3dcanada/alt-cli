import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-retry-1/independent/completion.json': 'bf3f7997701d78ab37a38983954a98f0dab459b316af5d1cb01f265bd3b17208', '/home/runner/work/_temp/campaign/8192-python-retry-1/independent/check.py': '7cfe9c2ae65f5d10222ffc9c8174dd52a209647cc96d70cd1ac90ec9abf5cc55'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-retry-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_90ea2f2070944df28be40f2618927c62'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
