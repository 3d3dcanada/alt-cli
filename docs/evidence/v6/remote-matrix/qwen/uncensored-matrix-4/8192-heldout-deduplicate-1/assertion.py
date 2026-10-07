import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-deduplicate-1/independent/completion.json': '4822c47784d35850830461956909c1402839b47fd284057ea77a2a6430d5f634', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-1/independent/check.py': '2e7cf99e81831b733f56e1683bed42212267a987984a80f5408c626151ad1c74'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_46dd9f22e3da430895e2e1dfed2d5806'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
