import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-python-feature/8192-python-feature-1/independent/completion.json': '3f7a042f984bd3be9324a913577ddfc05579f4fe1534080c76a28f11d02c2863', '/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-python-feature/8192-python-feature-1/independent/check.py': '5965e4ca6f7862175984d826c79653ee5167677b29ee38e2a9d48dc75e6bf9b3'}
command=['python3', '/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_3665bb174666456496c9f651af0c648f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
