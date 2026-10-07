import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-migration-2/independent/completion.json': '3b6252a8a8656942d917729520657591260b4844dd749fb49d0c280b9867d0c0', '/home/runner/work/_temp/campaign/8192-python-migration-2/independent/check.py': '5a4a6edcc10605565a8ad095d027bf56eb342bbb74177cc8d3d4feae4be87a23'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-migration-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e01f2610f9aa4588bf4947edf72e7acb'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
