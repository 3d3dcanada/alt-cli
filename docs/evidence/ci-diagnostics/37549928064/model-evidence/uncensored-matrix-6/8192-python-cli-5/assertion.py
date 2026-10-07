import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cli-5/independent/check.py': 'a85d4c69858c81d363e0476ad394b7ce509dce9a92c8c918f83730390e39b3b9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cli-5/independent/check.py']
assert all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()), 'Oracle inputs changed'
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1'}
r=subprocess.run(command,env=env,timeout=60)
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if r.returncode==0 else 'failed'}]}))
sys.exit(r.returncode)
