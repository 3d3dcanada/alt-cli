import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-b07-continuation-24/8192-python-feature-1/independent/completion.json': 'd5e4bb6b97e8d81e71599c78382986b67479de96399bec675fe146d2ec19bbab', '/workspace/.alt-repair/mimo-b07-continuation-24/8192-python-feature-1/independent/check.py': '9963b7e0e3a690675ba4fff6421e3ef8c3327b839463616d6f36d206d8cb8284'}
command=['python3', '/workspace/.alt-repair/mimo-b07-continuation-24/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_aabf2b7655694c00824cf027bdd8f72f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
