import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-pagination-3/independent/completion.json': '0050dff1c33bb46aae7a0ced5e742d1937718d6629c0800e811f9438cd7dc6f4', '/home/runner/work/_temp/campaign/8192-python-pagination-3/independent/check.py': 'fba9f97151358ecb7472db6b8a8e877372010c056bef471f56524bc44c095a73'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-pagination-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e7d30f9f28d348d9a55592674809a55b'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
