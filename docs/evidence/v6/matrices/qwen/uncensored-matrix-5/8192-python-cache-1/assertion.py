import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cache-1/independent/completion.json': 'ce1c7ce36727a3cf6c1ff997a514d761f5fcaf2f3a47318aa13819d9d6652bf9', '/home/runner/work/_temp/campaign/8192-python-cache-1/independent/check.py': '7a6f2b6f9e54ef8f0efcf07482706bdf5d7c44615c59a594018e713b0aa45b16'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cache-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_53398764797040feb7a318121a3a9cb9'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
