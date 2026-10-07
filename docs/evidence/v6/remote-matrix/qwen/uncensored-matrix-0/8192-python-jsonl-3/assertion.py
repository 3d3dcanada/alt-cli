import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-jsonl-3/independent/completion.json': '5680625a799fe52f4de2ed4af9ad30c1933490615f6911223d6c6119c2199414', '/home/runner/work/_temp/campaign/8192-python-jsonl-3/independent/check.py': 'd3380cd22d2f0bd4aee668cf1c5eabf719a8b1e2f3bec5e04ffade5b5dba1bb4'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-jsonl-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_6ad34a7703434af7877cdd4486b748cf'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
