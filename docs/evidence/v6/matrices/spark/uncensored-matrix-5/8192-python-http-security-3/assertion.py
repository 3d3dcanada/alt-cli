import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-http-security-3/independent/completion.json': '4d2e4856ccdfc312a45b895f9a58e1f8c43b7057882ef502bccec2bfed1249db', '/home/runner/work/_temp/campaign/8192-python-http-security-3/independent/check.py': '845d8c83a456d0954d6f83b3018f519f6c19e048ecac855c9ffaca5dd2f1f36b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-http-security-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c126a82e48c3429ba9fd7aeebaa29cea'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
