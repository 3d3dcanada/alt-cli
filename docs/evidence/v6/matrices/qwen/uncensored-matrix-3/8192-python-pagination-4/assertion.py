import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-pagination-4/independent/completion.json': 'cdbdf8efaadef80ce6a4acb61ee9a4c6a1d08d91f4d47abe2bce0044a166f7bc', '/home/runner/work/_temp/campaign/8192-python-pagination-4/independent/check.py': '6a65a0f20f0413516a2cb95a2edcd867909be694f834e7e01fe2ce3ad0a9ff80'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-pagination-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_b02504d32ad0448ba462c4ccbff245aa'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
