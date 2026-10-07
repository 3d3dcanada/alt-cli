import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-paths-1/independent/completion.json': '16730fcaba27795c6a568f3a3d2bcd2fda99d41fc7a7667ff85f7284204a0fde', '/home/runner/work/_temp/campaign/8192-python-paths-1/independent/check.py': '09549ca280292f7afba39da9a475da4b2cd82eff4fab7eeff73cf2687156eac6'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-paths-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_838e57fdf1184ac9ab9b926ca5cf0d24'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
