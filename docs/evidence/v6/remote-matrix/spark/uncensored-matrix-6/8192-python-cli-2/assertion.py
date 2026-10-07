import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cli-2/independent/completion.json': 'd352728a159130e382834b8b7726a84afa32a3f325ed950ddb36fddf66150fc0', '/home/runner/work/_temp/campaign/8192-python-cli-2/independent/check.py': '6a906fc35a4902427c9c32f9400c8155eb77f72997c4794a6701b7f1d28d69e3'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cli-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_91de4d80f5374ecea2f978d06df730db'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
