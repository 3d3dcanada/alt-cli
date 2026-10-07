import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-3/independent/completion.json': '806d5fe8c0cb8fdb19eb71d132487b36ab8970ef57dd8ad445cc5843ccd36166', '/home/runner/work/_temp/campaign/8192-python-dependency-3/independent/check.py': '217f51788731a06be47f11038079264677a569fa3a6be130964896a77f5724f9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_6efce328044e43a2bf7f376121701aae'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
