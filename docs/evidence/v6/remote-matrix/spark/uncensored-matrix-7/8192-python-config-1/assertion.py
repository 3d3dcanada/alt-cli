import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-config-1/independent/completion.json': '9ff1cb2488b8c22240ed36ee610f9c79420731f55d538104a07ba60da5036523', '/home/runner/work/_temp/campaign/8192-python-config-1/independent/check.py': '3e7d5fc29ccbcd35caf590db49c799305fd043ab7fd641adf70b692aaaf347ca'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-config-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e9ceb1450af54d6b9ed418a273886bb0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
