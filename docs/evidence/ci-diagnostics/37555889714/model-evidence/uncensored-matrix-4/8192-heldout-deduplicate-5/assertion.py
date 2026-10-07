import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-deduplicate-5/independent/completion.json': 'aeb4ca30f75b84e75ad73cf07f2302d7d6701483a09cb4b98a6c0d93fc06fd5a', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-5/independent/check.py': '6fe6e21f81437535fcd98e740d916b5bde37da6e2048fb3aa4daf8b7f33ecb39'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_aaac39eac1964494a5d1dc8ffb7177d5'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
