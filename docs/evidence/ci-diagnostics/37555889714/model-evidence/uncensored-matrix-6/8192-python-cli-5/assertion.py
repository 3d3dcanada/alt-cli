import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cli-5/independent/completion.json': '526960e8f52b69c82330bde9c2a09ace37c9d5a2b779a263fdf401a749e93d51', '/home/runner/work/_temp/campaign/8192-python-cli-5/independent/check.py': '571dc6f6fa9b421bf32279d437c3dcebbf452bc23c22ed7d8420e7b6edd6d23c'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cli-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_30664d72b7794861a7116874fd9a0562'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
