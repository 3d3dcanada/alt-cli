import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-javascript-config-3/independent/completion.json': 'c4775a6244d295c9e5c6277c94045a0e86729a301839f70e02ba6ea8f717f287', '/home/runner/work/_temp/campaign/8192-javascript-config-3/independent/check.mjs': '34c6ec25b64852efafdd20a0bb749dacb6887a383b25ce7fff63477032ec255b'}
command=['node', '/home/runner/work/_temp/campaign/8192-javascript-config-3/independent/check.mjs']
receipt='ALT_ORACLE_COMPLETED_68af07aaeea3465ea1c71cc54f45620c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
