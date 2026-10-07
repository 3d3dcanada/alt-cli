import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-csv-2/independent/completion.json': '0c261730d0ec2da70c0810cd6441cac74ad14c3df63dfe2f2785f50c8fe3c708', '/home/runner/work/_temp/campaign/8192-python-csv-2/independent/check.py': '9d9b44675b7c9b0951dc57b2209c1cd13bbf0b0dd1498bc9b48a4b15893d7a7e'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-csv-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_17ab9d69dcde41beada2febf0954819e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
