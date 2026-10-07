import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-toposort-1/independent/completion.json': '76278e208620a025ce750240a2b50690b69f7431cba11093da6de92711e3c9ae', '/home/runner/work/_temp/campaign/8192-heldout-toposort-1/independent/check.py': 'c04348690cf578ecf10276844efb5a1e77f4f606fb01cf2afe994d7eb02ebdb5'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-toposort-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_7ec32a3242ea48f480e4b945818d7193'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
