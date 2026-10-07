import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-retry-2/independent/completion.json': '962a0cb8a5d384750adbcfa20e06b0143a6084148cd5622262857773a8d0a6d4', '/home/runner/work/_temp/campaign/8192-python-retry-2/independent/check.py': '075d3e02731fe0c2948a80048d082046bb5c6c0c29a5f4d53d193f4b1988799b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-retry-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_7f2d50c6a9d049268f0a4f3f0f85ae2f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
