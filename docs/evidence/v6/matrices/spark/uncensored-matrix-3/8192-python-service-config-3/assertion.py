import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-service-config-3/independent/completion.json': 'e6093b63ecee4e913e0e1d525ffd6f788ebc8ba662f7d6ec8efa4b77ee080e8c', '/home/runner/work/_temp/campaign/8192-python-service-config-3/independent/check.py': '973d9c1171a517fc8ccba5523287dad1ba2302039966c0951b8eba394635b03f'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-service-config-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_3f136cb3d6314fb68b74a6896c3a6ffe'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
