import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-deduplicate-3/independent/completion.json': '5d9cea1c42a88405302a9b52ff36557ea8e2022abf9da9796a6aad433b5c8ab3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-3/independent/check.py': 'cb4c77f3407d8c7771e22300f5d674c2a1ccd10847a5cde13d61d4774705b03e'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_fb23fd0e137b41a499554362daab5b08'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
