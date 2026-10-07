import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-service-config-4/independent/completion.json': '8c22a68044d19b31fc7420b442993d497277b1d062c9abfe35638c6a0a394ec6', '/home/runner/work/_temp/campaign/8192-python-service-config-4/independent/check.py': '43fbbcb761f6b2c80a8e0cb9e1dffd0c0a635ebe1d31ca2b789937bf3d1c0492'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-service-config-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_9ae289c56ba347ada96cc68d2a26cf82'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
