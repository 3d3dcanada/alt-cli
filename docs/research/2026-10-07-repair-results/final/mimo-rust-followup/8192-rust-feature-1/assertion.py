import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-rust-followup/8192-rust-feature-1/independent/completion.json': '58f5decad565ca29a2fb82df31f017a3a904976241c9619769efd126dece3794', '/workspace/.alt-repair/mimo-rust-followup/8192-rust-feature-1/independent/check.py': '2031c67078d27e026b2a2dea87bffd2edb570301987a5330f574bfeb1ba14458', '/workspace/.alt-repair/mimo-rust-followup/8192-rust-feature-1/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['python3', '/workspace/.alt-repair/mimo-rust-followup/8192-rust-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a76354c68b474e9eadc99eaa95c19135'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
