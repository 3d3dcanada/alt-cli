import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-date-1/independent/completion.json': 'cb11a2f81fae77140f920e7524e520f9e1d241c9467c6168a65fdc768840645f', '/home/runner/work/_temp/campaign/8192-heldout-date-1/independent/check.py': '0c6583df56b0115cb84398821c5db1bcf9752c8d5c8b3d6cb1543bac9ca1abc4'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-date-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_3e0d179888544518af7c777b3991b2f0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
