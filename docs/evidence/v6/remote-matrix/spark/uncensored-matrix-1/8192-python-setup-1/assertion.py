import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-setup-1/independent/completion.json': 'e0f3d5d1ef8a29d01f1e8f5baeea49d1f8ad964afe4681fb57bb2bbbf938fef1', '/home/runner/work/_temp/campaign/8192-python-setup-1/independent/check.py': '0b13895a46bf81481ff9acf63976041137ce9deac5c46bc640329495d38a78d5'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-setup-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4cdae274e61149118fe0eeeadb87e0ab'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
