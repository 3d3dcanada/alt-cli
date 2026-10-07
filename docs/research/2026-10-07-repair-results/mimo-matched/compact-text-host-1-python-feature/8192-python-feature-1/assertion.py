import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-matched/compact-text-host-1-python-feature/8192-python-feature-1/independent/completion.json': '19cc9410688d8353664d4f6a4515d21736055b3211be4fe920d565ee6276e774', '/workspace/.alt-repair/mimo-matched/compact-text-host-1-python-feature/8192-python-feature-1/independent/check.py': '2fb1bd5cb1f0d595b057d2ea0ec068eb3cd458880b5a42e234814eb22a9194f8'}
command=['python3', '/workspace/.alt-repair/mimo-matched/compact-text-host-1-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_69a43d9c6aa7449fa120eb7ca7a1850b'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
