import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-final-qualification/candidate-attempts/mimo9b-heretic-q4-rust-feature/8192-rust-feature-1/independent/completion.json': '4d8d1dc01bc4ab0ad414a556bd1e459fb28851745384e72e3dd49aab62aff4e5', '/workspace/.alt-final-qualification/candidate-attempts/mimo9b-heretic-q4-rust-feature/8192-rust-feature-1/independent/check.py': 'b74aa3846ac5e9e7925b796473b25b04d3263ac40bc8b40df933f8c265c37134', '/workspace/.alt-final-qualification/candidate-attempts/mimo9b-heretic-q4-rust-feature/8192-rust-feature-1/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/workspace/.alt-final-qualification/candidate-attempts/mimo9b-heretic-q4-rust-feature/8192-rust-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c56ebc3a47064b828c3e825c5c44219c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
