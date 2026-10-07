import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-javascript-config-4/independent/completion.json': '901f0875acc9d53ca75d4465a2a7cea2cfcb413f4866824ac0188c9a81bed8c3', '/home/runner/work/_temp/campaign/8192-javascript-config-4/independent/check.mjs': '3d7e7df430008a103fb1aec1b3d3db6426d667a2dbab9330730f0c02bfa808c8'}
command=['node', '/home/runner/work/_temp/campaign/8192-javascript-config-4/independent/check.mjs']
receipt='ALT_ORACLE_COMPLETED_0fde43722a224095815cb94829b5b439'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
