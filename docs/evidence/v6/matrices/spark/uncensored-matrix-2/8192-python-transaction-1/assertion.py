import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-transaction-1/independent/completion.json': '6286c76067cb73c4d95f7dea9ab79d8e04c2c3f07854d6d4afe2d8f5cc2cb84c', '/home/runner/work/_temp/campaign/8192-python-transaction-1/independent/check.py': 'e72d5a9bc95542d16076b56b775b8f34c72e13005ab216db8cb36bc8d688e3c8'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-transaction-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_3d9b901758f94699841c61f31cfc6058'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
