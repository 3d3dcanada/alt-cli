import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-date-3/independent/completion.json': '2c5aaddcade6f3c84fae22c9956f0b34288fba34fdf7b43dd0efb130958f95ba', '/home/runner/work/_temp/campaign/8192-heldout-date-3/independent/check.py': 'b52dfc7a39db34eb548aed9b4e83901aae854ae2fb6733a84725d6cb6e032e8b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-date-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_29806afa4cc143e39a1abb3332b8fd08'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
