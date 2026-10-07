import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-jsonl-5/independent/completion.json': '46ad50a161bd942afd3d41bb21bc4badc7f7c3a9777114f749324433b0ec992a', '/home/runner/work/_temp/campaign/8192-python-jsonl-5/independent/check.py': 'ec67d42263d898481569d65635ff76710ffa3230af6b342a3f52ed9cbe8c9ec5'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-jsonl-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_f2b45d1945b54f96aaf47239a868bb37'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
