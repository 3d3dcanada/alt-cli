import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-http-security-1/independent/completion.json': 'e4f6b322b03fc4edff3e5e9236bf6262c5bc012f50372ff1871cc1a21a9ed7fe', '/home/runner/work/_temp/campaign/8192-python-http-security-1/independent/check.py': '372985ad3bdffeb72395639c465c58babab29bfd497bfbec3d13365449fd37bb'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-http-security-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_03a4f7d6b61e41d39a9046ef965a9887'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
