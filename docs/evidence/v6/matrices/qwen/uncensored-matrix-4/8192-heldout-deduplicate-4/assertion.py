import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-deduplicate-4/independent/completion.json': 'b14a4035fe746fda28a058038b359d5eb05be500e862522e461e869ce9765165', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-4/independent/check.py': '821754d33677bbcb5e043e6f8f2a8577979a18629f819c5b3ddc77cc6b735cf0'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_04ccda70a1cf4a73a70533b59f6c7470'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
