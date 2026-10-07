import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-config-4/independent/completion.json': 'd8a190eac5d29606acc0bb1cea70dd4007c81237ed2a8ca511b2daf4f8090c94', '/home/runner/work/_temp/campaign/8192-python-config-4/independent/check.py': 'b0269889105741f3dec53baa29d09c29b670698d311cdabb0016556c8dba3ef0'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-config-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_381130975a68461fa67f73716e2c61d9'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
