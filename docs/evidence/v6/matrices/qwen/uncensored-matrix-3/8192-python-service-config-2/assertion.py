import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-service-config-2/independent/completion.json': '5d04f1a9b4dc37f7e41516514953087232fa0ac73e249eaa51bbfb8ca6dfd6bb', '/home/runner/work/_temp/campaign/8192-python-service-config-2/independent/check.py': '5de4dc12314d59889714acb719e92e217b5dd4ec01be5bb6bbf27ac6837a18cc'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-service-config-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c0417ee6e73246dca02adcb0c2af434b'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
