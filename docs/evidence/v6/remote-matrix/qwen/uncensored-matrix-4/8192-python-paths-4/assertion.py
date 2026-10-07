import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-paths-4/independent/completion.json': '7e6094b8186f9eff53fd6451ed7988f925bf1848fb50ef744ea2c55b8f47f73c', '/home/runner/work/_temp/campaign/8192-python-paths-4/independent/check.py': '535d186c0b3562ef134b081bdfe5c7cc1af55d7da25a7106088bdb9a47fa4cd2'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-paths-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ada0eff2953046d58b8012c0b0f79f51'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
