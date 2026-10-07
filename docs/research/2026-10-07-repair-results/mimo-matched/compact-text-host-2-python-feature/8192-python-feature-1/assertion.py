import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-matched/compact-text-host-2-python-feature/8192-python-feature-1/independent/completion.json': 'e99d7cd39c282ac2940c39fe5bf20be804f9e9c5b03fe67287761b81b743a6e6', '/workspace/.alt-repair/mimo-matched/compact-text-host-2-python-feature/8192-python-feature-1/independent/check.py': '1184a2e6b10ba78721429f11c5a5d5d93b6440fc984376d7787132734cbe435d'}
command=['python3', '/workspace/.alt-repair/mimo-matched/compact-text-host-2-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_6724d4c4747840029c94b22a3c3e374a'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
