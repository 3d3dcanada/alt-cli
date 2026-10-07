import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-api-3/independent/completion.json': 'fe47a0ae969cceb65a081357aae50b3e4c921832c460f5c602f37282088f48e0', '/home/runner/work/_temp/campaign/8192-python-dependency-api-3/independent/check.py': '1f121cabc20c0adb86bd4a2def46ed602497cc7d34d546b026e75a028f1de4ec'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-api-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_42b43a2370324292bc565bd23bbbd26e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
