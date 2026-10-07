import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-retry-2/independent/completion.json': 'ee5a931e7a635e4e763461ba0932370db01f9f1222d73239059a767be54651bb', '/home/runner/work/_temp/campaign/8192-python-retry-2/independent/check.py': '59a365394d2307e7fc80d0e366de36f2fc2ba9476e7bf43c1965974a8dac1387'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-retry-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_9f28eb64e59748d2a17394cb5fb4f7fa'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
