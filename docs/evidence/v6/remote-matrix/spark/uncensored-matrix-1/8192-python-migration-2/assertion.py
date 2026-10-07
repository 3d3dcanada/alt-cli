import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-migration-2/independent/completion.json': 'ab6a4efa86330ad00d3b796d12cef5113bc8642d6a8ba14f6c6fc3f1c9deb6e7', '/home/runner/work/_temp/campaign/8192-python-migration-2/independent/check.py': 'e9447145925f929a512a5e7c1a9e92217bc001a7de8c2b3804c627d5a85f23c8'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-migration-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_d8ddffbfb1244318b2d9124755a99da8'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
