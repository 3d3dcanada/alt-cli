import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/spark-compact-screen-1/8192-python-feature-1/independent/completion.json': '95db3077fc63bd89ca9e62e1195feb41e57b667f482196fa7afb0a2b1e6d7b7d', '/workspace/.alt-repair/spark-compact-screen-1/8192-python-feature-1/independent/check.py': '600195657eff78b8b384a083cf0e3eae91adf8dbafe7234715e11d245de978cd'}
command=['python3', '/workspace/.alt-repair/spark-compact-screen-1/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_39485d724c014f4dbee13dd87a4e0291'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
