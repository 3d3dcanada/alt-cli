import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-compact-screen-1/8192-python-feature-1/independent/completion.json': '2bc623845d9f77166e9320d3df1ec0ca9b6827eb8742d84263bef88d0c534f91', '/workspace/.alt-repair/mimo-compact-screen-1/8192-python-feature-1/independent/check.py': '96584a38a5ef7c2381dff439ca0131b76d2818f83dedf57d9e9c5ae085256f98'}
command=['python3', '/workspace/.alt-repair/mimo-compact-screen-1/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_319336e032b249618c0bcad702db19ca'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
