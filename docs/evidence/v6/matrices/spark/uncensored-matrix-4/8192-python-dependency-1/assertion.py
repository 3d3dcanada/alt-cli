import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-1/independent/completion.json': 'd40ba09922ebcd1bcb4da5fd7d2b0c9ed8df2b13622b88553f85eed43c15fc5c', '/home/runner/work/_temp/campaign/8192-python-dependency-1/independent/check.py': 'e9360ebd5b83c2d2ff60566b79c8045db5d66888da10ffba18cf3ecc14b579e9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_8ee6e0f6ec0b4cccbbc31986be369d74'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
