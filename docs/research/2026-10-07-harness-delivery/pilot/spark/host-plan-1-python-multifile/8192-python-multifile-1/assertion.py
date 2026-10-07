import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-python-multifile/8192-python-multifile-1/independent/completion.json': 'f6e701f96618c50b049c02ac195395e375b7b9c943f34a2108a92c00caec1f5c', '/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-python-multifile/8192-python-multifile-1/independent/check.py': '66c3f46ea59221f8d3f94790d099496a640a00f78c3df3b3f05114faed078740'}
command=['python3', '/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-python-multifile/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_f6b7223ada2a409cbd8249aea8e3f04f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
