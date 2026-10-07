import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-matched/classic-host-1-python-multifile/8192-python-multifile-1/independent/completion.json': '7bd0994d2e3cb5a07c5f054421489acaa4bf9f6d13b24d072854780068f0c998', '/workspace/.alt-repair/mimo-matched/classic-host-1-python-multifile/8192-python-multifile-1/independent/check.py': 'fea1d716bd1453a203a7d013712b790bfaff0372d620d61d82f169084b6c8484'}
command=['python3', '/workspace/.alt-repair/mimo-matched/classic-host-1-python-multifile/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e0d26bae97584699aff9bd545eae4261'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
