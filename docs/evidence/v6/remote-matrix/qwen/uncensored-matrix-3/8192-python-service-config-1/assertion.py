import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-service-config-1/independent/completion.json': 'af9879b5dda9bf8cbeb33c2abd6db545963828a069088486fa624932d36cb68a', '/home/runner/work/_temp/campaign/8192-python-service-config-1/independent/check.py': '455299ca116afb2ba2f8ed0cf41910e9d98912e46b18858a89e49ba7641a21f1'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-service-config-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_2363d4958488465390116b7ee7de83c0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
