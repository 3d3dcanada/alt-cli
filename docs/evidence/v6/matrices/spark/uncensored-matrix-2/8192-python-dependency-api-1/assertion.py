import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-api-1/independent/completion.json': '0e2ff06c657771cd5ffacfa66ad90b7a7631011f7f0b51ea8b025ec4c0cfe46f', '/home/runner/work/_temp/campaign/8192-python-dependency-api-1/independent/check.py': '3740436fccfb75e73104ce393ca111ceb46c5e32cdf9d34da1fb2d1068b682f9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-api-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ef252918bf424130a65b05c6afda0fcb'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
