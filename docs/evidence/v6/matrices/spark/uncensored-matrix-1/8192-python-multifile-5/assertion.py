import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-multifile-5/independent/completion.json': 'ad9dac434f006a98824573029b027062ec836fbafde746bb5a9238567cb1a284', '/home/runner/work/_temp/campaign/8192-python-multifile-5/independent/check.py': '61810c8e29fc40ea4acc9e7f2f3e4e6b42f5dfe2907fc99cb266d55f1845cbc5'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-multifile-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_bfdd944da12743369e02e58a009bdcd4'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
