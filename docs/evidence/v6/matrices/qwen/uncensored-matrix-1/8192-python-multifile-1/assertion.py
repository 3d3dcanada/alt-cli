import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-multifile-1/independent/completion.json': '1665cd357e4d26e7caaa11fad0df8b1c528123276e2c52dc2e0ef28538449d5c', '/home/runner/work/_temp/campaign/8192-python-multifile-1/independent/check.py': 'e9a6461c5c5502a1c672c644c4b522bb511bb83e28fa633b9c7acccfa5482b94'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_44bdad337fb043458c12fc3d00ac8bf3'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
