import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-rust-16k-followup/16384-rust-feature-1/independent/completion.json': '102d278be659f66201eb107d6dca2bd4adf3047a90f014ad320dbaa97b35bcbe', '/workspace/.alt-repair/mimo-rust-16k-followup/16384-rust-feature-1/independent/check.py': 'abdd6da53fefa3b5cbc20da3aba78c5d0a24c2c44ad153ac4bb801833fb8b4ea', '/workspace/.alt-repair/mimo-rust-16k-followup/16384-rust-feature-1/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/workspace/.alt-repair/mimo-rust-16k-followup/16384-rust-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_dd13760fa1c643a7a7480d550ef0f9be'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
