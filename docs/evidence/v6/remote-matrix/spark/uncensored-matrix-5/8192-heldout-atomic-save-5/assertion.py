import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-atomic-save-5/independent/completion.json': '8bd896c4a1465a230a60e903046f213ae9051bbb3316835eaacdb733f5fbf7d3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-5/independent/check.py': 'd264a652d881cfc27796b1f205616fca03217c135392aced44079fbc64a87f25'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ad6293e8227b4fd892af43b43b789428'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
