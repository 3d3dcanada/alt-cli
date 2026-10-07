import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-python-config/8192-python-config-1/independent/completion.json': 'eddbad61568799b81771b205035360c8155634e7105c4af274a29d9647b929b4', '/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-python-config/8192-python-config-1/independent/check.py': 'c1c30646ef1f20960e14104b37339b1b3985c0332049f628810722d6dabb75b7'}
command=['python3', '/workspace/.alt-harness/qwen-workflow-pilot/host-plan-1-python-config/8192-python-config-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_3eac6dccebaf455bb8f3ce216d72be17'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
