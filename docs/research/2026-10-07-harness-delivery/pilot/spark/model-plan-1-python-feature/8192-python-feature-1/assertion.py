import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-python-feature/8192-python-feature-1/independent/completion.json': '25dea63360deeaa4b33697ddfa9e868907d97763771ba9edbd746c53ab1b3db8', '/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-python-feature/8192-python-feature-1/independent/check.py': '58127fcd79bbb837c3130d7ac4a3f02899fd028821dd1b64d0b3cd104989ca93'}
command=['python3', '/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_7978e72661cd442891a07d2730e655d3'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
