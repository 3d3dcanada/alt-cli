import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-jsonl-1/independent/completion.json': '50f94aebb6555719ed16072b78bdd3f784d570048d1e708f0e3790ac9c534848', '/home/runner/work/_temp/campaign/8192-python-jsonl-1/independent/check.py': '6df23582843c91322e92e71e3d2c684f797c33360aff8189ff34f2a5eed9322d'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-jsonl-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_abba7396eb894dad8a1cba0b5fd9d74c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
