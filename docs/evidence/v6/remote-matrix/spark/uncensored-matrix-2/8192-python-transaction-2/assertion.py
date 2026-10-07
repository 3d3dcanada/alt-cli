import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-transaction-2/independent/completion.json': '10b1adea159508fdf0f07e087767333a3ad95dc3f3ac4d9edfdb994ee87c4c64', '/home/runner/work/_temp/campaign/8192-python-transaction-2/independent/check.py': '9bf16f84e853576c7d9ee5cb62542d213a7a4c1f2bf7c7519e7e2581c795be85'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-transaction-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_47d000a9fff24670b68af92b24437fd1'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
