import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-atomic-save-4/independent/completion.json': '01b28a9c005fa3786a0050d04ed52f024668496199ae19593438765e4494a625', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-4/independent/check.py': 'e3810f2f696edfd7e7701b72ae88ca431e9e20b5441cf0319f74b70fecba9b04'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_19839be74a354b509668f5091316f9fc'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
