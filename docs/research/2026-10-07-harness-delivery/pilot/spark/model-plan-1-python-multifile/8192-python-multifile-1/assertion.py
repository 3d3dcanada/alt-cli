import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-python-multifile/8192-python-multifile-1/independent/completion.json': '7a81b022f0c5a83e127821ab57be84bd69d76b28a3c4451f826da554fb1254c9', '/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-python-multifile/8192-python-multifile-1/independent/check.py': '246a2b129025f6fc6ec46e44ee5f8a78fa0c1e27be4ede93e23f8751452dba9c'}
command=['python3', '/workspace/.alt-harness/spark-workflow-pilot/model-plan-1-python-multifile/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e5069d367f0a487a8de1cebb1dbb8c83'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
