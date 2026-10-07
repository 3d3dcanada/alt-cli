import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-javascript-config-3/independent/completion.json': '9920dc52f07a548d278dc68bd33259b0e005fd7786f40db67b23eba2cf70e5df', '/home/runner/work/_temp/campaign/8192-javascript-config-3/independent/check.mjs': '0fde1695560eabf5044f32d167b0ac1ed3be89cda1eb87793b89fda4a59a59ac'}
command=['node', '/home/runner/work/_temp/campaign/8192-javascript-config-3/independent/check.mjs']
receipt='ALT_ORACLE_COMPLETED_9cb17a9750334769be8e64e67b1453e2'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
