import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/completion.json': 'a424ae93c275aaadc481bccde5f10188159698d0088a56ebea3c616a95661114', '/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/check.py': '55e68997a09ac324ed5699b06fca752d1bb743ec902cf176e86abcc94ad0034c'}
command=['python3', '/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_d4234358ef474b158ffd8a5d5fd7ccd3'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
