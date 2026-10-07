import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-feature-2/independent/completion.json': 'f1d71bee3e3251bbe0a937fb1b5f81eb2c1e7d4e580e8a11e92b22cb918d894c', '/home/runner/work/_temp/campaign/8192-python-feature-2/independent/check.py': 'a0ffdbb2fbe733cc476f34a4633012a726298701ca2fce3167cb0bb09354a089'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-feature-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ae7ed2ced34d468596119c53deeae92c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
