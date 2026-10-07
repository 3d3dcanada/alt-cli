import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-python-multifile/8192-python-multifile-1/independent/completion.json': '4e29996843b59513e861ecb7979c5ee9318fd8826d87d32e4e09fd36caf07fdd', '/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-python-multifile/8192-python-multifile-1/independent/check.py': '9494199781cb5a20d841c54006f22bd14d947d2844b49c14af58fe3d6eb04667'}
command=['python3', '/workspace/.alt-harness/qwen-workflow-pilot/model-plan-1-python-multifile/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4bafef0a5c0b4a4e87938eca9b65340a'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
