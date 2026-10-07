import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-rust-feature-3/independent/completion.json': '226f988651c106ccdb8d24cc23c5b290b7ea4e2ed5b90bc9dfcacae05a9d4927', '/home/runner/work/_temp/campaign/8192-rust-feature-3/independent/check.py': '1cfabe298a9878c0b910c6e1ca01b740bcc047c413385634255fe1b29faf77ec', '/home/runner/work/_temp/campaign/8192-rust-feature-3/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/home/runner/work/_temp/campaign/8192-rust-feature-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_592bc68786ca4bb398848f3887e91ba4'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
