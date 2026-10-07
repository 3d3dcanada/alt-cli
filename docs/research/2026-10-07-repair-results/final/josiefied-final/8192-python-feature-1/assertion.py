import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/josiefied-final/8192-python-feature-1/independent/completion.json': '0b05895e064fc7f6475e53977e2482408a7ce3b5a56ca631e09fc5ae95843503', '/workspace/.alt-repair/josiefied-final/8192-python-feature-1/independent/check.py': 'f783ce3fb632db2ee63428153ce444f03d318e888e4805a9edf2cfa6fba3898e'}
command=['python3', '/workspace/.alt-repair/josiefied-final/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_5889e8e794aa422d8dff392db2e13848'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
