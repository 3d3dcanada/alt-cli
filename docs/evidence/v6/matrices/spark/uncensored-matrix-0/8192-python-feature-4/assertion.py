import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-feature-4/independent/completion.json': '020fdd4b2ea88432b8371fbba5aa55ce6036ef7bf04832e9fd33f8d504fe7484', '/home/runner/work/_temp/campaign/8192-python-feature-4/independent/check.py': '08f8858c9c76d695ddd15d69bb2333e8d57b1e71dee9cbf8940fca22d7c26de7'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-feature-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_354f047b5b2f4fd0aa497ce1638a2b71'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
