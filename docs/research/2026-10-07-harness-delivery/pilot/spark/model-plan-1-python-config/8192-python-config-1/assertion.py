import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-python-config/8192-python-config-1/independent/completion.json': 'c94379b124961666d139f1d580bf8d8c068f6a13ba708b1646d243666ecf79e4', '/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-python-config/8192-python-config-1/independent/check.py': '4692f28596988688166b0c534aed2a219b6d691dbee93b34e16972625543a0b7'}
command=['python3', '/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-python-config/8192-python-config-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_41a67b81606544b4bd8b8c70629a3300'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
