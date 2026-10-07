import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-date-2/independent/completion.json': '450bd3144e2d998cf32e45158548f16e1d4aeb9e56091153b927b303d9be7027', '/home/runner/work/_temp/campaign/8192-heldout-date-2/independent/check.py': '24247ab31815997705f0eb9cf28f070c9437bed8da6820c62c4bc4ac53cf3af9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-date-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_5c13bb73415144aa926c7ea499623c76'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
