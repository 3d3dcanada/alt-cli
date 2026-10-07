import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-5/independent/completion.json': '84e9b693998b0970ec5e6920cb90f1076289ca5072827c19cad7332c224438c2', '/home/runner/work/_temp/campaign/8192-python-dependency-5/independent/check.py': '7d372f75860af497178292bab06ae01f5e351117624f164e40fe0ffdaac51821'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_dde3b18515d24002a80368a792d1e9a0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
