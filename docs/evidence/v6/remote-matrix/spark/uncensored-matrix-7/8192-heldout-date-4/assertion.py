import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-date-4/independent/completion.json': '24d6fcadad6a1578d2f2dad1dede181571ed979d96a9b76094c770167bd52094', '/home/runner/work/_temp/campaign/8192-heldout-date-4/independent/check.py': '7b7175348d11feb8fa72480d02f899814345311604f77ff2664bb1c7fc67eea7'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-date-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e5f3628546934863ae4b895b96d6b29f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
