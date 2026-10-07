import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-http-security-5/independent/completion.json': 'def2bb2f0cd15d20e9349acc915fc4e324a3733d5f87d5b4fc61d06999ef5d2d', '/home/runner/work/_temp/campaign/8192-python-http-security-5/independent/check.py': '01221e13bedbdcb47ea76c73e15dbd83e68dfa7a94f82ee1449d1d14c3f50147'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-http-security-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_233c39fadcd04c3cac377c6ee5009619'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
