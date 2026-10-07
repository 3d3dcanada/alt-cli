import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-api-2/independent/completion.json': 'cd2dfa9ca4601fa4c4cafe9b7e580de8640705ee4f83fdbb7e3f38df3ff90ee4', '/home/runner/work/_temp/campaign/8192-python-dependency-api-2/independent/check.py': '40531d523196b8bfeae3d88f48bdf15b133ca143d3f410b31f300dd527d57a3e'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-api-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_fd7430f447484f7892a2d439750e8b29'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
