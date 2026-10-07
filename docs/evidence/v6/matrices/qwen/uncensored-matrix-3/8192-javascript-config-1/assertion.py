import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-javascript-config-1/independent/completion.json': 'd8f2f1c9a1035d33deacb3f76ca663f8e471f4de502797cb0ef602f397bab8ce', '/home/runner/work/_temp/campaign/8192-javascript-config-1/independent/check.mjs': 'f374294d80ccf9e9ed1304e4e055098b33115741831e8810c02a04cdced97edd'}
command=['node', '/home/runner/work/_temp/campaign/8192-javascript-config-1/independent/check.mjs']
receipt='ALT_ORACLE_COMPLETED_be7270b7737542db8b440b6ed68eb367'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
