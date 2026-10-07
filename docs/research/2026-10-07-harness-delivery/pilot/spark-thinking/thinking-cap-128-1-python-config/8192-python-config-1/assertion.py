import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-reasoning-pilot/thinking-cap-128-1-python-config/8192-python-config-1/independent/completion.json': '50b682d8527197b1805addecdfc6883ac65c0e70f63e31d2efb17528891a6b09', '/workspace/.alt-harness/spark-reasoning-pilot/thinking-cap-128-1-python-config/8192-python-config-1/independent/check.py': 'e2f0475e4048d5a09e8bae628a1131814731f2580ff1f79b78a634b57609d67a'}
command=['python3', '/workspace/.alt-harness/spark-reasoning-pilot/thinking-cap-128-1-python-config/8192-python-config-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_3d968cec255b4954a11cc9654296eed1'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
