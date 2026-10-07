import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-paths-2/independent/completion.json': '657c0d363d850a0053f8fe2e2a4b487c7b9d4ae8d5be27736732f436a37b071f', '/home/runner/work/_temp/campaign/8192-python-paths-2/independent/check.py': '606bb7ded7d57111a510cc702b000f1b2ccbcd9b34b2d1eeb2b4a505b83b7f3e'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-paths-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4d372941b691433dbb3b2f1f91140056'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
