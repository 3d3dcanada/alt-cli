import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-rust-feature-2/independent/completion.json': '8fbec3523afe8faa20d4750df52cc8f110b189d0ae43f5796040b3d01fd542be', '/home/runner/work/_temp/campaign/8192-rust-feature-2/independent/check.py': '0f6d281efa3f83ad87a4ad025354a0b507361d284fdd7ee881f4497602832a66', '/home/runner/work/_temp/campaign/8192-rust-feature-2/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/home/runner/work/_temp/campaign/8192-rust-feature-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_029799d2dbfb4d56b673ece39042f5f8'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
