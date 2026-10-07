import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-toposort-5/independent/completion.json': '7725d0f327957d7167ad5f0378aaab65ea734904e13cdae7debe0f58ebe502f2', '/home/runner/work/_temp/campaign/8192-heldout-toposort-5/independent/check.py': 'b94bd64cba546300cb732a5b84ad714a48bb6363be6a9c0d6ba0186d6480dc9d'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-toposort-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a7345d01df3c456cb1b561385a518f88'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
