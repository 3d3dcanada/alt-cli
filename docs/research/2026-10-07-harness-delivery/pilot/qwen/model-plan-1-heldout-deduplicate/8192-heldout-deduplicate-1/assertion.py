import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/completion.json': '6c5ac71c26d63e00cc55dc6e6b9be87c961d41ffe8555359eb7f856b8c0fb271', '/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/check.py': '05f6dad8ac3b6ec06ae5a61921d7f6bf54d203a154c424c8176a8c6c4587c0e4'}
command=['python3', '/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_05f45b4668b949babe86ecda2a7ed282'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
