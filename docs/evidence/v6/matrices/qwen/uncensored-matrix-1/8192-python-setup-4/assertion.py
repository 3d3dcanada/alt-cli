import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-setup-4/independent/completion.json': '08b8cc42d1c2d727a2e2e8d25ee44818af7c5e58a33b24c0afd4bb71aa5995bb', '/home/runner/work/_temp/campaign/8192-python-setup-4/independent/check.py': '6d0d59ffbd89be08c1c4d0df01e4975acfb76f5859f0ef666dacf85688c59cc8'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-setup-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_de9824c73f3c4f118a3c47530c86e67b'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
