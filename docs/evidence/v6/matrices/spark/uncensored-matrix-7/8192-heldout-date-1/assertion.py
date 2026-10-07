import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-date-1/independent/completion.json': 'f4680a71e9ceb32c78c7596d5a35f272b5a0d35084f104856272e8a05276eb17', '/home/runner/work/_temp/campaign/8192-heldout-date-1/independent/check.py': 'a7aad265d9f55f5484c847436c81ff0657a0a7c89fed8ebba4fb7df1f442cb71'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-date-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_d8cb56e571f440378ad8fa9ed36460ea'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
