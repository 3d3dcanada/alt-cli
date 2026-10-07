import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-matched/classic-host-2-python-feature/8192-python-feature-1/independent/completion.json': '6eea470a77223aa4295e011ca4d9fbf6d2a160fe541051281a8a96d9a997cdb2', '/workspace/.alt-repair/mimo-matched/classic-host-2-python-feature/8192-python-feature-1/independent/check.py': 'd89071754d7217169073fc90a04c9867ebaed7e8dc00ddad21c3fc3cff3dd6ff'}
command=['python3', '/workspace/.alt-repair/mimo-matched/classic-host-2-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_1c4a124a9af844208d4866bac7f759d0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
