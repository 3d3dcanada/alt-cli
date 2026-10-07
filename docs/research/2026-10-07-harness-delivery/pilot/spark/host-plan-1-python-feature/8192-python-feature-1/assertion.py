import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-python-feature/8192-python-feature-1/independent/completion.json': 'ba68170eefda514e482fd3c69eea97b9df306f34b090a437dfb5bc16104ef478', '/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-python-feature/8192-python-feature-1/independent/check.py': 'b77bf14db74ffd08d9a4c503fcc2593cc9500cb5c9092d7a9b3327f49143d515'}
command=['python3', '/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_06b733807536492db040bff6a3480cdf'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
