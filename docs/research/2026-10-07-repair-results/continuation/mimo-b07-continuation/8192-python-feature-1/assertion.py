import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-b07-continuation/8192-python-feature-1/independent/completion.json': '540fd359939cc04452e732b335314a93afda3c96785dce4c64e2a689c4ee28fa', '/workspace/.alt-repair/mimo-b07-continuation/8192-python-feature-1/independent/check.py': '06d8c6bc9aa148766971c8a26411eb011c61c40fa3929a59246178e60d90fbf0'}
command=['python3', '/workspace/.alt-repair/mimo-b07-continuation/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_299138d5c3224ba99cf3be426a3753f0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
