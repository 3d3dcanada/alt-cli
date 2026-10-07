import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cache-4/independent/completion.json': 'ba53ec3c7bcd64a154237dc006d248745f45687cc8da3b97c59db66484686d0e', '/home/runner/work/_temp/campaign/8192-python-cache-4/independent/check.py': '0da54cee18749046d76fcd8d0a3da79ef6f38a35c35501b338ca7d491c2ffd9a'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cache-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_52d1ac1b7d264d3eb9c6339c5c10ece1'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
