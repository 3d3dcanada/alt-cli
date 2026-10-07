import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-pc-ready/live/mimo9b-rust-configured/8192-rust-feature-1/independent/completion.json': '6f461751fe2ccf9b2f27def9c06272d6866a371c17d74e3ef8fe6eac40337045', '/workspace/.alt-pc-ready/live/mimo9b-rust-configured/8192-rust-feature-1/independent/check.py': '1b3f9c03a0a3d95ed8984cbc941341939acf5c4252201956f46525cad05ce91f', '/workspace/.alt-pc-ready/live/mimo9b-rust-configured/8192-rust-feature-1/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/workspace/.alt-pc-ready/live/mimo9b-rust-configured/8192-rust-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_0f07d5ed34a54f1392cddfe630402ac1'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
