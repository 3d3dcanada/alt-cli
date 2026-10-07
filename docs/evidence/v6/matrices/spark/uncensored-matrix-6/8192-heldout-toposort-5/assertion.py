import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-toposort-5/independent/completion.json': 'a3325763f055e20742b8af5da22c039eacc77d44f17dba9a047aa40f10654479', '/home/runner/work/_temp/campaign/8192-heldout-toposort-5/independent/check.py': '043188f2bd5c4d99085e7d23d94b8c2c795bd9ba10ea9510ad1b6563956f35a1'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-toposort-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_d79b5c04f5584d22b3395bf49f59a382'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
