import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-multifile-2/independent/completion.json': '32d7181047b89662ecdee6cc6a7eee1a373ad95f73c85f47a4574cad3b633c69', '/home/runner/work/_temp/campaign/8192-python-multifile-2/independent/check.py': 'de73f7221337b1ffa477d417d068caf898508465f0af930f6451bae7fb715089'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-multifile-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_272be493a7214fcd9f165c18b642fb9b'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
