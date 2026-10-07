import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-migration-1/independent/completion.json': 'e56e7ea8890427d2da631bf76f58d8f3f26cb7092a39fcf19aca6818fee0ecae', '/home/runner/work/_temp/campaign/8192-python-migration-1/independent/check.py': '9de5c2b896681b406e848cac98ae5fc08dd3f9c7acf19dfb52b465e3ebab77b4'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-migration-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c3af7fb03f30402caad521a3352ea65d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
