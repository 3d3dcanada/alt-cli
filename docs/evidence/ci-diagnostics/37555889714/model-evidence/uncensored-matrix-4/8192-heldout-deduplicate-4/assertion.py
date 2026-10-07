import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-deduplicate-4/independent/completion.json': '1f18516da17e6ac6c0a574472765916d69c356c07addba4044a55cf0998662bb', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-4/independent/check.py': 'fcddb4a9e69e37a1b4cc92531bde50e6e3b7500b2ce5761740969529e3688d75'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_05e5dcc0dc6d451e8709790eb60788cd'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
