import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-retry-3/independent/completion.json': 'd8f84379fa74193035779ea75e8c9d00ab2a5a9fc444ed5c03db7ca7f9b48637', '/home/runner/work/_temp/campaign/8192-python-retry-3/independent/check.py': 'e0b5188d0067aec8f734926d4d785ce1ade4239749033a8ed339a6121bd78b72'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-retry-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ca5f8da112064037afb2871c8e33ad4e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
