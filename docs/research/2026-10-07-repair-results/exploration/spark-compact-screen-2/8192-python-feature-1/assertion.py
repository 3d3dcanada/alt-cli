import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/spark-compact-screen-2/8192-python-feature-1/independent/completion.json': 'c9f4869e1f7b35547df83ef7a94a0f7b826a55b61c83cb815761bb55f7425404', '/workspace/.alt-repair/spark-compact-screen-2/8192-python-feature-1/independent/check.py': 'd29bf6efa4d750ae4cc310c344d2cf8cf8c93b0e5aac68d42e220d04d50a3ab5'}
command=['python3', '/workspace/.alt-repair/spark-compact-screen-2/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ec1bfe7a52a248deaa4817223ffda86f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
