import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-api-1/independent/completion.json': '5b90c24b202f39bda43da76e0086a15a8916b6c31e5c0d7950aeeace040a1863', '/home/runner/work/_temp/campaign/8192-python-dependency-api-1/independent/check.py': '15363f8a69080208b5d848a0560549799cb21940bcab9f7501de1e3bfe72e470'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-api-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_b112f0eb33ec4cb9a48f3512ace92f7b'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
