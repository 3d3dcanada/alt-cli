import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-python-feature/8192-python-feature-1/independent/completion.json': 'a9af16ab0c52fff513014ec476b24249f4d0971a28c82e4fc7e810a5784bfc60', '/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-python-feature/8192-python-feature-1/independent/check.py': 'ed8b49fa4b7c24e69bd92999d33a6c5dc0661413a80aaadb34530be6ea76691a'}
command=['python3', '/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_b9fd20638b6248b7a5b6c51dd20ad8b2'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
