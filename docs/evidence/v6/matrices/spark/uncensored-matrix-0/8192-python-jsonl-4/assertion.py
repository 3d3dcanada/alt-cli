import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-jsonl-4/independent/completion.json': '24cbb215456674a3ccfcc79d9915f6ed05777e72598c838e20bdfe5a9b7b52f4', '/home/runner/work/_temp/campaign/8192-python-jsonl-4/independent/check.py': 'ca47e0ce243419a41a4c21c558064e9c15aa6ee751f3021d518f4ebd5127d134'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-jsonl-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_8de51d0a02fb48adb7f8881e46840f0d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
