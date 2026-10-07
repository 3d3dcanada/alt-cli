import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-async-4/independent/completion.json': 'a884f7298ed2299e6466c711dae6921d8e2d6b204597e2007620195ce473ff6e', '/home/runner/work/_temp/campaign/8192-python-async-4/independent/check.py': '672462320134cde09b0c0ae7e597e5f338f4d3a98dcf2778bc4271dcae7f4edc'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-async-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_f179197ebed3497e8bab7b3ced88047c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
