import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-matched/compact-text-host-2-python-multifile/8192-python-multifile-1/independent/completion.json': '488593933c9b8207556e789c1ab2d511f55c631ce00b15d54b3578df7ad90bb3', '/workspace/.alt-repair/mimo-matched/compact-text-host-2-python-multifile/8192-python-multifile-1/independent/check.py': 'bb66b63c4b18ab0d297d36df4360a70c4a97bac6835935f731c3da860cb33e92'}
command=['python3', '/workspace/.alt-repair/mimo-matched/compact-text-host-2-python-multifile/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_6d3c6c9a31a44f428e1d70e0dbc365d3'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
