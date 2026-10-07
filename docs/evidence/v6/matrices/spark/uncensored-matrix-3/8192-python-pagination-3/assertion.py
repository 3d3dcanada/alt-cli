import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-pagination-3/independent/completion.json': '511bf899170c2e205f682baf1cd112716e374f5b7f9669b8bab39f0c5f65da09', '/home/runner/work/_temp/campaign/8192-python-pagination-3/independent/check.py': '6baa64c2dc1ae295637b319b520772c4d6d601825602e470b831d40eef13ef8a'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-pagination-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_245846e5e68f46aaa6ce129e08720467'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
