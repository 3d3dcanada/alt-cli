import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-csv-1/independent/completion.json': '7acb1dc2500d99119a98bf82e92185a1b45d28064c9b48e8cb8a6e6d08b2bf2b', '/home/runner/work/_temp/campaign/8192-python-csv-1/independent/check.py': '241e92e6df9e3f9354fd9dcd358270d9f17c286698eff36ed33ef3ccbff0a1d4'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-csv-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4b4757be52a04b999e4e3ff97df93a16'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
