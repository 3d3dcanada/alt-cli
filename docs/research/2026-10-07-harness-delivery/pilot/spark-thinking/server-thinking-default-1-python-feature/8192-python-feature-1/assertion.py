import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-reasoning-pilot/server-thinking-default-1-python-feature/8192-python-feature-1/independent/completion.json': '9b38b9849a908bf7d9341932a6558a1bf7e355ae1496455813326348cbcc1dd9', '/workspace/.alt-harness/spark-reasoning-pilot/server-thinking-default-1-python-feature/8192-python-feature-1/independent/check.py': '541c86861861fc860b43c5517de4886bf841c13653c52aa676a745a19c655f9d'}
command=['python3', '/workspace/.alt-harness/spark-reasoning-pilot/server-thinking-default-1-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ca5a7ba8892d429eb055ec2bb5746e07'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
