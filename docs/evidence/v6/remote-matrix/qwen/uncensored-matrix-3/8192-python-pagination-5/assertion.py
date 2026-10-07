import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-pagination-5/independent/completion.json': '37e55b59d767dab35fcbe304b6c1336927c423d75badc6dc35bc43ffe6c8758a', '/home/runner/work/_temp/campaign/8192-python-pagination-5/independent/check.py': 'aa8270f8d842596e0a2c0a4d12c8598598680476bf8eb5c3baa1c072af230f3e'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-pagination-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c101fd7797a14585b519d92eafe16db3'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
