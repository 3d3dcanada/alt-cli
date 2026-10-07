import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cli-4/independent/completion.json': '66e05512c8bc720e5c1c0e4bfc40ecdd63b5e74931dc1b081c97c49f7809815e', '/home/runner/work/_temp/campaign/8192-python-cli-4/independent/check.py': '6782ac159c7b29de8ddd75fbcba96e8311d5df471f3267cd1e4cc2903aa5ea36'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cli-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_1dc80ddb74194e51a534a55dd3d3eee8'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
