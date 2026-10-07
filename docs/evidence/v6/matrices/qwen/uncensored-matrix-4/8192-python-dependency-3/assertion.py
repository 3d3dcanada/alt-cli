import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-3/independent/completion.json': 'a047f08c1eeffaf28b4abe452207347e36b44dbe2f3c5d380f24ae5285919d94', '/home/runner/work/_temp/campaign/8192-python-dependency-3/independent/check.py': '229d956289abd0624c1b5ed56bac10ecee16bfba46711516037316050c4c571c'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_659ac1fae958435c916a1069b72b620e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
