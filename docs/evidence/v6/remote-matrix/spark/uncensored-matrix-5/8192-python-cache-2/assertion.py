import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cache-2/independent/completion.json': '0f8e523573dda0a12825e115183dd11dbab9cee5ccd093d9c54f85af288b23b4', '/home/runner/work/_temp/campaign/8192-python-cache-2/independent/check.py': '68a24c72871f4f0781cc0924e436079a0723d420bbd2b4130c7d2ed049dd97fc'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cache-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_9d3051278ff04e5b932eb459f358c4a3'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
