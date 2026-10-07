import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-http-security-2/independent/completion.json': '0defd06e0973d2003f7f63bde0ee0d4a107e0624593dfeb6d8a3d81d8b2b0580', '/home/runner/work/_temp/campaign/8192-python-http-security-2/independent/check.py': 'f756f128f98ece366a54748b27bf5361619beaa62ad9cdb82c871fcc33e6552c'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-http-security-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_5150902684f0421b8ce1e53de8ad18ad'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
