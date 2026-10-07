import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-paths-4/independent/completion.json': '0fe6ba02a0040e9c8b57a390c3305fa857c2e3e62cf6724b106c56b07f7dcbcd', '/home/runner/work/_temp/campaign/8192-python-paths-4/independent/check.py': '95168a313353a81a68af8e7e4557b049b9468c777c42dda7302074945b575a89'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-paths-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ed2e2442ff2c4c2aaabaa4c790130088'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
