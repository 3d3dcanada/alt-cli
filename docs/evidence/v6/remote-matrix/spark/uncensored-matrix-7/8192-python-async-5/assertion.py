import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-async-5/independent/completion.json': '2f4e271ae6b8755c104ef2424f5ff1970693bd0cf85556c442e07f3b5682da26', '/home/runner/work/_temp/campaign/8192-python-async-5/independent/check.py': 'e1832e7e503cd2b43788c5cfbcd838cebf88de99f5b48dcf2a54c59428c48f82'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-async-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_94252c3d977a4d0fa63650887e5abd09'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
