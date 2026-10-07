import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-migration-1/independent/completion.json': '6d1ceb258ab80bfb81400204534663907777dbeee2da5afab51e413756aff6f3', '/home/runner/work/_temp/campaign/8192-python-migration-1/independent/check.py': 'b52d0018b32e4f1d50725e11437ddcd2aa6fbcdb7f45c8779cea79e41633c50e'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-migration-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e3561707692840cc900854856ed8ddc4'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
