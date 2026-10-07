import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-rust-feature-4/independent/completion.json': '611180d57514e40f696da45bc7d43ffa117246b1e2ff1bec007d46243740c32f', '/home/runner/work/_temp/campaign/8192-rust-feature-4/independent/check.py': 'a38e8c7f598d8bbf5c97dfa1bb98c7fa991d2069408984e0856e3f0e04eeface', '/home/runner/work/_temp/campaign/8192-rust-feature-4/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/home/runner/work/_temp/campaign/8192-rust-feature-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_84c0537aba4c4fffbc831e2e7c7fcf23'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
