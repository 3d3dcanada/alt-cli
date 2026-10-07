import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-atomic-save-1/independent/completion.json': 'bf2b014bf93d6c49ffc68719e64ec7a8da375c2f41b76216cdb97ae53108a749', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-1/independent/check.py': '9f41e2c99cacb81be924ffe875f1ae947dfaf98711bbc4a63f4c46a15f67f025'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_06e7f48951e546638a7ac1f2a59aab0c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
