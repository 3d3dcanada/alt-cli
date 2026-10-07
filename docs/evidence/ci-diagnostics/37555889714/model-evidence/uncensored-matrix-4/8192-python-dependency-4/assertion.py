import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-4/independent/completion.json': '4b91728a5d4e4cc309fcfc8e9e1e32da9edd87442865342b8e5862a4a383909d', '/home/runner/work/_temp/campaign/8192-python-dependency-4/independent/check.py': 'd40e792ad0814bde2ded7aaabc405e5abff0b247560f258d90003934a0ed7e6b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a0720fb03ee448f484120e5c33b02161'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
