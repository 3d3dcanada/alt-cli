import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-async-2/independent/completion.json': '83e590ceef24ade455676072cc02f36e774979a09cf37309158339b1b91add3c', '/home/runner/work/_temp/campaign/8192-python-async-2/independent/check.py': '8a0898248c1079ff9705ff6704452a3de518e1a9ab95ff1dd3800ec102bc4dd0'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-async-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_049f32651d2a47ebaf96175caf445fda'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
