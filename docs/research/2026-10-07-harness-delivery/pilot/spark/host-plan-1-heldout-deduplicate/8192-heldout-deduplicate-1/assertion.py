import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/completion.json': 'a734c4b05537457f3f7066458aa7effb27e277b74586d42cadc61b618c0aaf03', '/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/check.py': '207ba20951ccd1e4508e32f9b342ab79ebcbb27490270aff4968c35f70aedf9b'}
command=['python3', '/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-heldout-deduplicate/8192-heldout-deduplicate-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_b8b051cd299549c296c2490779c1096e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
