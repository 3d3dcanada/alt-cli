import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/spark-reasoning-pilot/server-thinking-default-1-python-config/8192-python-config-1/independent/completion.json': '809724f5195cc0918078a30b5beabab91e88075fe26313793d32b3824d214d4d', '/workspace/.alt-harness/spark-reasoning-pilot/server-thinking-default-1-python-config/8192-python-config-1/independent/check.py': '86d8c00843fee3097528e2825fe4ab9dbff606c87e71e5934c4adb56b5f6cb99'}
command=['python3', '/workspace/.alt-harness/spark-reasoning-pilot/server-thinking-default-1-python-config/8192-python-config-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_bef2743a7d984b64b926bad9882314f8'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
