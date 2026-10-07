import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/josiefied-final/8192-python-multifile-1/independent/completion.json': '8d91831635f843da88815041bfa8bef19e08474f3b59827635016216340548ff', '/workspace/.alt-repair/josiefied-final/8192-python-multifile-1/independent/check.py': 'aa43f0095cf7125fe836cd1157bc596395b756e3bd6e9d000dcbbfdd5c4342d5'}
command=['python3', '/workspace/.alt-repair/josiefied-final/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_240447d6bc374ff2babfc636aa77474b'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
