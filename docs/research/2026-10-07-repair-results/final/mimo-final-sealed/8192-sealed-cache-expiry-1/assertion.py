import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-final-sealed/8192-sealed-cache-expiry-1/independent/completion.json': '6d9c851a7dd53caab7dbb428c9a24e1ecda2f0a58e7790cedfac7b9a899f55e6', '/workspace/.alt-repair/mimo-final-sealed/8192-sealed-cache-expiry-1/independent/check.py': '3fff78e5643ee3dcc38bf7874203b347c2bd2f705730fb41c5008845812f607c'}
command=['python3', '/workspace/.alt-repair/mimo-final-sealed/8192-sealed-cache-expiry-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_2cbb2737b29140a184cc2d0bd969f067'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
