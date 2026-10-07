import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-final-languages/8192-rust-feature-1/independent/completion.json': 'e92869fdbcb982b7193624526b1e59fbd645da50992e78506eae3743ae4b9792', '/workspace/.alt-repair/mimo-final-languages/8192-rust-feature-1/independent/check.py': 'e62a481159d8b5463cb9f3e3945aba92937da34ee211b889b94ec13bdc886bd6', '/workspace/.alt-repair/mimo-final-languages/8192-rust-feature-1/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/workspace/.alt-repair/mimo-final-languages/8192-rust-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_ff85b93ccd2b4f9fab40f8dae5e7e19d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
