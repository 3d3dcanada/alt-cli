import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-date-5/independent/completion.json': 'fa7c43834ab5e87172aaac12fa9a01d720c9ffd9bf89766530c8e4ffec26c8bb', '/home/runner/work/_temp/campaign/8192-heldout-date-5/independent/check.py': '36bc87c30aae1bb9ad9f75550adba8018ac04c85299209d881d00e522f020810'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-date-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_dda03669791644e1bbfda05c07f65ded'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
