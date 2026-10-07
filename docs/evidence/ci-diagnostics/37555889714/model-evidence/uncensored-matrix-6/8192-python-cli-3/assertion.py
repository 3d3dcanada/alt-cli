import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cli-3/independent/completion.json': 'ee4188bc645890e44bda14f064b2bf55e862a7a1eb699f79f9f87769863e59d7', '/home/runner/work/_temp/campaign/8192-python-cli-3/independent/check.py': '7b29cee803d6a4697718f4c18e65411cad982a8e451ab3320ad4408d74466dfc'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cli-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_015fdb5da6954cbcb028327748acdcbc'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
