import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-feature-2/independent/completion.json': 'c6eb2472a2477c0fdc6818b1e11e570b23fd7556a7e4b7ff419c157c9e979e4f', '/home/runner/work/_temp/campaign/8192-python-feature-2/independent/check.py': '58f6d615f78eac513ee5795f1e5d2bc1714d832ca16f2031d7b4c5559563d930'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-feature-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c3cc1bd9e3ed4abba5ee607e3e94b12a'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
