import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/josiefied-final-lines/8192-python-feature-1/independent/completion.json': 'bda9c08e3318e92df56d35106c36bd6e7ecba1cfd1ce3335302288e48801b76a', '/workspace/.alt-repair/josiefied-final-lines/8192-python-feature-1/independent/check.py': '18a0950d9901c91778b0b2de0703d78efc5535919ad0ef0203ec2bdcaade4bff'}
command=['python3', '/workspace/.alt-repair/josiefied-final-lines/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e03ce20f4afb42a785cf6b6621c9e2a4'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
