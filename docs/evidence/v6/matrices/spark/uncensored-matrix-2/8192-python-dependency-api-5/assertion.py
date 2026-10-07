import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-api-5/independent/completion.json': '9e6f20d9a9f18df3ca74aed892cf6745f75f9ec2167b3494e8cc157179a86cdb', '/home/runner/work/_temp/campaign/8192-python-dependency-api-5/independent/check.py': '01515b22790e448a17e0be4fd0847a4fed920e5c4a642d9f340c0d59149cfb80'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-api-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e9221cd8ba2b41d69657c28e8ddbee92'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
