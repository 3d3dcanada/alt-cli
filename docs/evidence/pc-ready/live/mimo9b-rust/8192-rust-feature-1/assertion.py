import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-pc-ready/live/mimo9b-rust/8192-rust-feature-1/independent/completion.json': '356e512bc031a1ef132c4aeb57fe2989f1ade2df8b5719fcaf4548491152dca8', '/workspace/.alt-pc-ready/live/mimo9b-rust/8192-rust-feature-1/independent/check.py': 'ec5c3d7076ec7a606135cc9445a4932802db73d50f8d13c641bae714a2f5085d', '/workspace/.alt-pc-ready/live/mimo9b-rust/8192-rust-feature-1/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/workspace/.alt-pc-ready/live/mimo9b-rust/8192-rust-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_6920b172ef7b4c64b5b047fda11c7ed4'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
