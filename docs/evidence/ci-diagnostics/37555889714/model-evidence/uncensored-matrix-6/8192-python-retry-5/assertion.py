import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-retry-5/independent/completion.json': 'ebfe121550c89b05b98a0c0e601fcf0e893f336e918ae40ec25cd3c51336cebc', '/home/runner/work/_temp/campaign/8192-python-retry-5/independent/check.py': 'ee66b5400a9749a71f07559db8bad7c8e84205ed58091531147b122edae6b669'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-retry-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_f88bfd91fd5b47c19b9feb3e7d0f7089'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
