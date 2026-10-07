import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-paths-1/independent/completion.json': 'dc274035c12973feeafaec7e53170beb5a254d0b7e81ab8788f7faf94bf89686', '/home/runner/work/_temp/campaign/8192-python-paths-1/independent/check.py': '7fbb5f7f5b818ac479cdfcd19c82c76dff9fcc271e0a15f5be500af01484003d'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-paths-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_1afa47fccba94b4db260813c5d2dcdc6'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
