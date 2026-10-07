import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-setup-3/independent/completion.json': 'bd6bfd98a5e088d35b3c57c663f18cc61af9380a27129912eb5faa3a4a1eb86d', '/home/runner/work/_temp/campaign/8192-python-setup-3/independent/check.py': '21b3d4df9b5d089b117789c6b6c4b8384e3e89de66417a88a2ecdfa03d3f8d85'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-setup-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a8f6107752504b87baa432fd93289dca'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
