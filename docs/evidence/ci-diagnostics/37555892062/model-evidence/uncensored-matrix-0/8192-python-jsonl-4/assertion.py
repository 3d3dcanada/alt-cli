import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-jsonl-4/independent/completion.json': '309e1c2fbce1a15dc21683c6d29d858ebbb2b347d095c1297720b7458db724cb', '/home/runner/work/_temp/campaign/8192-python-jsonl-4/independent/check.py': '59dc09508435f1c9ee5c832032009960ae2857a2b3a3830ec8c799c7afe37617'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-jsonl-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_2d01db4157cd40e0900d5567df76a617'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
