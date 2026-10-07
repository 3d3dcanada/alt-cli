import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/completion.json': '49768b711b0fe4ce8ed8f02e54edf8fd81ece4224a5df3d829f2769378f015b1', '/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/check.py': '2e97d9d3db5b339ab301e5b26ce2bd1ee9c7617bc85c559597ca0e83e699d7d0'}
command=['python3', '/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a9a90fbed29e4f13853ae2317276152d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
