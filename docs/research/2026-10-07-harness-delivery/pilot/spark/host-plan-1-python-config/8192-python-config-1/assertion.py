import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-python-config/8192-python-config-1/independent/completion.json': 'dfa2f388c4d7a1ad513b3672a3ff3ec37afe0212000bddff392d1b9bc4eea902', '/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-python-config/8192-python-config-1/independent/check.py': 'ea738c00dbe1219cef07cbe73199f8c41ec1ab0d7ef3d184800edf85ddedd594'}
command=['python3', '/workspace/.alt-harness/spark-workflow-pilot/host-plan-1-python-config/8192-python-config-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_50206fc6189c44ed990f7bc448560a97'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
