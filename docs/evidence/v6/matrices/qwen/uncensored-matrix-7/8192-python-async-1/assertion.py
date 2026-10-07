import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-async-1/independent/completion.json': '43916ca64feb67f840eaeed608588657c5c8bd74ff399cd45cf4cb99d91b261f', '/home/runner/work/_temp/campaign/8192-python-async-1/independent/check.py': '8cb28d585957a34902ecf70c569cf4cd2a92c7ed77807991b63798b9fab5aa93'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-async-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_cee4da8a652443f196880dfd54bb6178'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
