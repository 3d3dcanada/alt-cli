import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-final-qualification/candidate-attempts/josiefied7b-abliterated-q4-rust-feature/8192-rust-feature-1/independent/completion.json': 'f8d900180117c7d0bf2728d9026128c8fb3bf92a63f63cbb82268d12a4ee3fcf', '/workspace/.alt-final-qualification/candidate-attempts/josiefied7b-abliterated-q4-rust-feature/8192-rust-feature-1/independent/check.py': '1ba883b43d35726baa74b71766c33fa47307dedcf8160b50f9d0ae0ca94455dd', '/workspace/.alt-final-qualification/candidate-attempts/josiefied7b-abliterated-q4-rust-feature/8192-rust-feature-1/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/workspace/.alt-final-qualification/candidate-attempts/josiefied7b-abliterated-q4-rust-feature/8192-rust-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_5a79a355a67d4e50ab79f5f0648c3564'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
