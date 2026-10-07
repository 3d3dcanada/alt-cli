import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-final-feature/8192-python-feature-2/independent/completion.json': 'e881fec7fa4dcc84cb8f3f9e7424868b8ffd4d2a7b55bf81ab854a4c4d7b26a4', '/workspace/.alt-repair/mimo-final-feature/8192-python-feature-2/independent/check.py': '00e4943925a707eed69408bfb8cdc1b8f65e9dfa3682638f2ef456f0fec89cf1'}
command=['python3', '/workspace/.alt-repair/mimo-final-feature/8192-python-feature-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_6a122351c9f1470a98926ec8da536e81'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
