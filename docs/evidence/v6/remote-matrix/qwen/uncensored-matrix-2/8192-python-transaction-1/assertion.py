import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-transaction-1/independent/completion.json': 'afff0f5f293fbbe44311b9b2a2eb13067db09d95c5816e045746679ffe4eb7f6', '/home/runner/work/_temp/campaign/8192-python-transaction-1/independent/check.py': '3c2fe20011cbcc3984474ef392dd0e53ff99318a1f6ee9ac29efb84693e26b98'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-transaction-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_d314aacd88fe490baf47c6d0bcd14621'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
