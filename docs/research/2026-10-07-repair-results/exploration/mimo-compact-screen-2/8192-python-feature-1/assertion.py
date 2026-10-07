import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-compact-screen-2/8192-python-feature-1/independent/completion.json': '8d7bca702539b1b25be0435e93f749f8edc62fbbf6d687b7d67e6e18e8a78b7f', '/workspace/.alt-repair/mimo-compact-screen-2/8192-python-feature-1/independent/check.py': '5bc11a12bb04b613ac0019bdb7bf56170c80c6708787f5e9f9591ed54f152224'}
command=['python3', '/workspace/.alt-repair/mimo-compact-screen-2/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_b929803ad8f948b6a105ee1012160a7c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
