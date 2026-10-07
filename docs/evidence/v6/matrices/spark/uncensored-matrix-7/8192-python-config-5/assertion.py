import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-config-5/independent/completion.json': 'c7222a56d10ee71031b22cb65aa65c8cd2561709b41a0d6f3c3bec6af9955ac4', '/home/runner/work/_temp/campaign/8192-python-config-5/independent/check.py': 'ff5bbeec66f99caf315a4ac66d1bcfac88f75fffbe12a17fa7bd32d2007ce1c7'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-config-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_6bd18783ca0040b0972fbb162202ee8e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
