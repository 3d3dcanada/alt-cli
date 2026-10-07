import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-matched/classic-host-1-python-feature/8192-python-feature-1/independent/completion.json': 'c367ac3bd641a03b05c51138bbe52a4f260a03b883b4c1b9bb07b2f33317dfe9', '/workspace/.alt-repair/mimo-matched/classic-host-1-python-feature/8192-python-feature-1/independent/check.py': 'c08c5eba99fa04110ef8d66078c2c2b5969fa944b78933d70864ea11a47792ec'}
command=['python3', '/workspace/.alt-repair/mimo-matched/classic-host-1-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_3d6d6bef7628482b807686b5e0f3ffd4'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
