import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-deduplicate-1/independent/completion.json': 'aa848edfc8dd6dd5b71c9d2372db9c6cf58c0dfd8bac478d5ecf7d86474b7ecd', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-1/independent/check.py': '656dc921169f4c71585ddaa32812e115a0745314257003ed5a7cad0ea7fe5a3f'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_7693bd9d4b7143eeb2232061c741329f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
