import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-multifile-1/independent/completion.json': 'ca01f7d7a55ec67f1d151b01484b1417efef5da1b600f5906643b323d94d9846', '/home/runner/work/_temp/campaign/8192-python-multifile-1/independent/check.py': 'c7430b5f16aa07f8b03f0aa0b9c18e12692516f331f6e59d27e994a4f15ac810'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a7041a11badc4449b85b48cdfbd09f45'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
