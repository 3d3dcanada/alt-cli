import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-csv-5/independent/completion.json': '325337966beff0f3aa34d7fbbdfe09ce161e4488a8332e26be69e20b04800e3d', '/home/runner/work/_temp/campaign/8192-python-csv-5/independent/check.py': '2a485639cda3ee9edbbf299f8974850f8774b341d49ae27b3a9ccdeeb66576d2'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-csv-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_44b49426bc7f42d0a8bbcad267c8486d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
