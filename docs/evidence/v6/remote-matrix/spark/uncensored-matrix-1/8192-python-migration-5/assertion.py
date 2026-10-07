import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-migration-5/independent/completion.json': '3f567be3a4c4c069844b622b4a2195daa5771e4a92665db67a8bc1ce47debc9a', '/home/runner/work/_temp/campaign/8192-python-migration-5/independent/check.py': 'cbc8f0783e6d148ed03c80c0b09d5c36e20b62f3393bf36282a8b23ff959562d'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-migration-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_99cf195362704e17a93774310cfed670'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
