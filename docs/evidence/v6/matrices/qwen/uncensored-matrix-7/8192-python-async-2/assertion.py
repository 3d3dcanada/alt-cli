import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-async-2/independent/completion.json': 'bcb654c71813a94f627b5ff5598240dcd59f3ffb48be04bc6bacbc16b6fc9dde', '/home/runner/work/_temp/campaign/8192-python-async-2/independent/check.py': '9b424ffc83a9d3268fb2652f0d9551482dcd1f2ccd9a53028d315e88e097127a'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-async-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_73b8978ad51a4aac86985e0869a3ba32'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
