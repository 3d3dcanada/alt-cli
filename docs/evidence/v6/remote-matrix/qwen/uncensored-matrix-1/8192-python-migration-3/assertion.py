import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-migration-3/independent/completion.json': '010a83a221dcf17653f9f101f225b1b2781872d94a5dd33d34acf38552eb91f4', '/home/runner/work/_temp/campaign/8192-python-migration-3/independent/check.py': '6b47bcc598916abd3fd99529ded34e676b7ee0d09f5725517cdcc6817276d826'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-migration-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_9e619e1f250d4cb4be665631032001d8'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
