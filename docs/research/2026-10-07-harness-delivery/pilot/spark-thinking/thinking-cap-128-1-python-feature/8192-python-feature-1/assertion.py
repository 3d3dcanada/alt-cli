import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-reasoning-pilot/thinking-cap-128-1-python-feature/8192-python-feature-1/independent/completion.json': '5d927cb0acd900d6a941fc0fd3d18dc494e62a57a2f8e39994c245465b450c63', '/workspace/.alt-harness/spark-reasoning-pilot/thinking-cap-128-1-python-feature/8192-python-feature-1/independent/check.py': '03b812bf1d72b7228833f9f7cc06db665e043f3e15a76c8ee9b0a21c813cb9ca'}
command=['python3', '/workspace/.alt-harness/spark-reasoning-pilot/thinking-cap-128-1-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ae6a55abf0ad44e0b1c21fa6f3fb59dd'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
