import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-csv-5/independent/completion.json': 'ee6f9241fe04e9141262de6b1e6fbc930aa65110a0f59048a8114f23dd477805', '/home/runner/work/_temp/campaign/8192-python-csv-5/independent/check.py': 'e938c30a632b7267ea670ec2737229bd3cca1900d2da3c7d9ab39910db67e09a'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-csv-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_21b6e2a00a42423ea7baf6b2a5b67b29'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
