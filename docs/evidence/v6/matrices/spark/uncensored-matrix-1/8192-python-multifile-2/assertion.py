import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-multifile-2/independent/completion.json': '98d3e3a1ab41c16ca7691d8c7715c3177b878d76f7552b2a09e2a69c684f1067', '/home/runner/work/_temp/campaign/8192-python-multifile-2/independent/check.py': '6ce5b9945626388bc17941299f8f3d8438c5b83438f9155d2abf725c0baac37b'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-multifile-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_7995946c75c94ce2860fe10417fd61d4'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
