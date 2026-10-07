import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-pagination-1/independent/completion.json': '68e9611377227814b3662e8bacca703ecce6095261a582b4427f47e09df9a509', '/home/runner/work/_temp/campaign/8192-python-pagination-1/independent/check.py': '99d46366d9bf21689e713d66fdea9df27802c312533204b7b05bf5463bb86cde'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-pagination-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_76a842f8537045d1bdd72907b9a70b12'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
