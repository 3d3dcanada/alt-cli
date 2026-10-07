import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-transaction-5/independent/completion.json': '8d9143a85ad42a5c4303c9025bf134e6a07c3776275907fe6af320b363a6694e', '/home/runner/work/_temp/campaign/8192-python-transaction-5/independent/check.py': 'ff48bee05aa1d7ac2482cb39ebb9f6e2728a2e8ad57e07b0e32419b2a18784f9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-transaction-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_21756b48ea7b4abba9acf1a153974a25'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
