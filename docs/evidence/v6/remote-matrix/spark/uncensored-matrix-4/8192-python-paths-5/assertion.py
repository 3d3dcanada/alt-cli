import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-paths-5/independent/completion.json': 'a99f099c5443b5bd5dd4e1fd16800b67cab804294e4bc88fc5d28ccb9bf68f2a', '/home/runner/work/_temp/campaign/8192-python-paths-5/independent/check.py': 'fcbb23c42222a157bfa2ad286c906b12aeac2de5087dee9ba34bc2f910b069f1'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-paths-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_9034aadb4a33465aaa80ee3a0a7229b5'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
