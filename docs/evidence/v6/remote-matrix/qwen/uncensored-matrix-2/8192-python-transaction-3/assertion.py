import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-transaction-3/independent/completion.json': '02ffeceb23aef89945d7585d2668b4b1e17cb49cbc112b8a12b2b42a097bce86', '/home/runner/work/_temp/campaign/8192-python-transaction-3/independent/check.py': '5c27b5fc2f58fb31ef8d3edfd586bc4d5418851a8e2fc007a834902d69327421'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-transaction-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_91eab52bb9ee45c0aab4663e62ea65de'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
