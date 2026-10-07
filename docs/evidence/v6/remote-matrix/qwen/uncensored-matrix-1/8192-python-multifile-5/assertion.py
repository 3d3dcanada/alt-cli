import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-multifile-5/independent/completion.json': 'cd7143d55fd5eedbbbf09836bbf2c42a4581d815b095b12ea560f5ad80c22d4c', '/home/runner/work/_temp/campaign/8192-python-multifile-5/independent/check.py': 'c18dc7d970d74d04b5f78d46427d45b42411fedb7dc96c53d986001c9d54aee0'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-multifile-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_2004b3d9ad6744ee95bb3277442d26a8'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
