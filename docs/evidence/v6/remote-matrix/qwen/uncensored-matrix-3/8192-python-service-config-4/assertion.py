import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-service-config-4/independent/completion.json': '495e3e221d072b89c407f503a331be35ee46968d3f065b5e4b6444c8563bc21c', '/home/runner/work/_temp/campaign/8192-python-service-config-4/independent/check.py': 'b7037f87abbd68087853fc26aadf52f98c80798abc2426b8a749ab5345ece635'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-service-config-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_b51a069d2ee448b0af9f15140d5284c7'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
