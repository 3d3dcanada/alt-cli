import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-paths-3/independent/completion.json': '1953467fd3080442a38b7f2e074c949b742acbee57916778e01e19504ab42acf', '/home/runner/work/_temp/campaign/8192-python-paths-3/independent/check.py': '44c5fbbccc0388c83667900a3ed40a69eace8b6c250fe55075f45053cb44a893'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-paths-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_7bce427d89f14896b1a807ef1ce982a6'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
