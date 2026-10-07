import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-matched/classic-host-2-python-multifile/8192-python-multifile-1/independent/completion.json': 'ff2cf46dccb73c9d55e3cb22be49b3922b0e45c54c9c71622b74671c0065c455', '/workspace/.alt-repair/mimo-matched/classic-host-2-python-multifile/8192-python-multifile-1/independent/check.py': 'e2792e1f3fc47605d768a9bf75713f691a6183762eee94fe1517eb1b37a5178d'}
command=['python3', '/workspace/.alt-repair/mimo-matched/classic-host-2-python-multifile/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4534dc5b8e244deea66e768d0c11e8f7'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
