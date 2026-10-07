import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-python-config/8192-python-config-1/independent/completion.json': 'ae976ba82d9aec0edfdf3e4bd632977ad6a1fd4fb7b39f198de7a5069d133a54', '/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-python-config/8192-python-config-1/independent/check.py': 'd925f681e90a75857af0ab234f729c88b19c57bd217bc0b8946df138395bfffe'}
command=['python3', '/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-python-config/8192-python-config-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_f3c17a32a0834edbba27759e3539888a'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
