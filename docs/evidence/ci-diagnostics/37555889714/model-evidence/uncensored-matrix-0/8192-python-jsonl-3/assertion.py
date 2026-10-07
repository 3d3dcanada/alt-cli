import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-jsonl-3/independent/completion.json': '47296a3c9bb7f0a1401de83d1310b0c31280dc90de50daf3f3ca71e2f4907ea4', '/home/runner/work/_temp/campaign/8192-python-jsonl-3/independent/check.py': '9db59f302a602c2c26ad54d2dc2192d11e539be17a19c4474dabce8e02001d86'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-jsonl-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_5db23f6d44264d32bae08ae9949de6b1'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
