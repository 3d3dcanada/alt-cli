import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-migration-4/independent/completion.json': 'f5c41cef3c6fa03ebe6646df7c85c307a1757afa0cf4f68813e8ad487e362453', '/home/runner/work/_temp/campaign/8192-python-migration-4/independent/check.py': '672a3519d7e95b471e3d924eb90cda45d886a7f3e04f083a8ef01f2520116d87'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-migration-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_d3df1c74bd8a4b058854cb1db43faab2'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
