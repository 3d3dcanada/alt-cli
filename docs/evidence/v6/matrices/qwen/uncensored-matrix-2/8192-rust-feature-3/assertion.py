import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-rust-feature-3/independent/completion.json': '81b282227ffc489dae4744a8bd6f3fdecdd0dcd3ed559ec098aead40b548bdea', '/home/runner/work/_temp/campaign/8192-rust-feature-3/independent/check.py': '7a14fa3b8d88206f3b11e6c763001bc5a339db3cd2e3146fcd600e304b4e4d84', '/home/runner/work/_temp/campaign/8192-rust-feature-3/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/home/runner/work/_temp/campaign/8192-rust-feature-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e83adc7af4a54d44b723e2ee7745ed4e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
