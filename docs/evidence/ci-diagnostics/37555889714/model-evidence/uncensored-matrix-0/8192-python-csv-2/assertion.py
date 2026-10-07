import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-csv-2/independent/completion.json': 'f8f9e8cb36bcf4001ac6c64f9aea8b0e0edcf56af4ff6c103278688e8ce0c8c6', '/home/runner/work/_temp/campaign/8192-python-csv-2/independent/check.py': 'f3a4d741d965fe61d3675ff5957d7cb2547deee878218bafca63eef011f46d28'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-csv-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e564cafc224c4612bb14055721df4ed2'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
