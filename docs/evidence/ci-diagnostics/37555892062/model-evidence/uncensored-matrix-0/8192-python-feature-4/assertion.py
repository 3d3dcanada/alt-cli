import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-feature-4/independent/completion.json': 'a2b48e62a61bab31663b3c4d4cff7fee236b3dbc684703c950a9051e762bbd30', '/home/runner/work/_temp/campaign/8192-python-feature-4/independent/check.py': 'fcb2abb3ddcf55f11d7ed030802f7afc530c09a11470a3a3bc297c65a1dcf506'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-feature-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_786e9ac1baf24a9e8a4390d2049fcee1'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
