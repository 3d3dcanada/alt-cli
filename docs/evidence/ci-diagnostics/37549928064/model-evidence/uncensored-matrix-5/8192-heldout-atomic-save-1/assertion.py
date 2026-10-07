import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-atomic-save-1/independent/check.py': '6a4e1db0dd228cdcea4f599de0b71ee51b83165034ba02e6d4d7ba38daf74d70'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-1/independent/check.py']
assert all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()), 'Oracle inputs changed'
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1'}
r=subprocess.run(command,env=env,timeout=60)
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if r.returncode==0 else 'failed'}]}))
sys.exit(r.returncode)
