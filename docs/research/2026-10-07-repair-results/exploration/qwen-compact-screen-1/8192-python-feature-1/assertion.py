import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/qwen-compact-screen-1/8192-python-feature-1/independent/completion.json': '18d8fb9446beb48b85628ba9a7c739afa88fb49f7632e6fa21ec48742b15af63', '/workspace/.alt-repair/qwen-compact-screen-1/8192-python-feature-1/independent/check.py': 'e730a0a1edfbe29857fe284798445f10c4d5fffc81517a96df70d86156aa0961'}
command=['python3', '/workspace/.alt-repair/qwen-compact-screen-1/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_0a5ca5e794514378a5c28722bfddaa37'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
