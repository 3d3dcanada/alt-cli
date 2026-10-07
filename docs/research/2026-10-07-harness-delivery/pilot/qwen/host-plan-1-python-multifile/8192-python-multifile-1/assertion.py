import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-python-multifile/8192-python-multifile-1/independent/completion.json': '61b22c0f36bf62a696641c585eb1dd066ca15b1477cf71e5cbe08adb502cfd9f', '/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-python-multifile/8192-python-multifile-1/independent/check.py': '3d0c6e2eaa9e0b34f8c571b056e69b3560b583e25362f33e6875de86015d0720'}
command=['python3', '/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-python-multifile/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_03d30fb2c6084dc09874504064996b3a'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
