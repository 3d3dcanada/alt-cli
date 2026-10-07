import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-csv-4/independent/completion.json': 'aa5db533a50ab708aea267779d0a9687aece2de636c9eab59beb5245fd9474ed', '/home/runner/work/_temp/campaign/8192-python-csv-4/independent/check.py': '7f14d7fb453bf8d96eea115fbb4ee04a5fbd501d4a5631693c7b6a82853e8ce0'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-csv-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_62080625d4a64251b2db803f68d38071'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
