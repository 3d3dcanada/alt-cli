import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-1/independent/completion.json': '2feaca60715ba32fa25c3175ab5bc5968fefc3b20299d2a6ad1ab139117ba1c8', '/home/runner/work/_temp/campaign/8192-python-dependency-1/independent/check.py': 'de8db6704989ab4856fbef0b02ee61b88255b29e9d7fcaa332bd667b4f2cf6a2'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_74ce9dbb602b4200b6f0fc8e3c1c5b41'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
