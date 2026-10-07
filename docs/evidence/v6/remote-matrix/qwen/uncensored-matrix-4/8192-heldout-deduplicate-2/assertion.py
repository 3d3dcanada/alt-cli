import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-deduplicate-2/independent/completion.json': 'b1ae21253553061b4c9346d832af30764665122d86ad65ecee7040679ae12e74', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-2/independent/check.py': '81dd17d2f6a6307a54659956cade8ec408adcf96a2cd5fd478320843f4609ac5'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-deduplicate-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_115634c2c55542bd8e811f89c2a2b675'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
