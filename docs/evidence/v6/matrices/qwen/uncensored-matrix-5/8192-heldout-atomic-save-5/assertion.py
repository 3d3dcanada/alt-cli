import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-atomic-save-5/independent/completion.json': '511b5f805479edb75400f6f52ce26b89a2323508acf7d437724459af6bd45bb1', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-5/independent/check.py': '7325329f18bf65b14f0cab4f4f07f97f4ad50c063df4917743ed72f0f3ff4dde'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-atomic-save-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e8fd240e3b2245068138faa865d30a57'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
