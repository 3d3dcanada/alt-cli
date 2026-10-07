import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-rust-feature-1/independent/completion.json': 'fa53c830f02e4c513dd2c487e81dd70bacced77e6219a0ed2957e20a1d168010', '/home/runner/work/_temp/campaign/8192-rust-feature-1/independent/check.py': 'a2f30d778a669ff16832a45fc8c774981e44c4357bebf7d21da7751f6345f3c7', '/home/runner/work/_temp/campaign/8192-rust-feature-1/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/home/runner/work/_temp/campaign/8192-rust-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4b13d211d4b547468d22e79bdc242d64'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
