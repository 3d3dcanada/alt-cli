import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-deduplicate-5/independent/completion.json': '2acd377add4a3f0500ee485a62f6ff591937efe3c4f9d31a4127705b9ac62752', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-5/independent/check.py': '9f6409e1a4918ea4ba2be2e1274b3ac7621bda782c051bf7c0ea531190aa250d'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c815cb4166b744f9afc9dc065a25fd8e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
