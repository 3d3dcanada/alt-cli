import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-date-3/independent/completion.json': '58a7b716f73b1d4b5b4940b97b42d6bfcb5829991c0755b67453acf8f277f4c0', '/home/runner/work/_temp/campaign/8192-heldout-date-3/independent/check.py': 'a38d3fc87084ad793f5ca91cfd0544e5b178fd14c53eeae6012a7838aca27981'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-date-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c7257414332746d79e57b7d64f5e857a'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
