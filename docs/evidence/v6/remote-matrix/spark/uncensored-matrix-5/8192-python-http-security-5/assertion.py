import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-http-security-5/independent/completion.json': '68cf001f2533a4d7379075947ecb57330d84d02254d89bf88153d1c71caea963', '/home/runner/work/_temp/campaign/8192-python-http-security-5/independent/check.py': '3a432e203652c0adfdf44b551b3ba62e39aeb9975cfe93c7d8e1c56bcce7e02c'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-http-security-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a747c977d8a34a2db8ab9d7e4fefe55d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
