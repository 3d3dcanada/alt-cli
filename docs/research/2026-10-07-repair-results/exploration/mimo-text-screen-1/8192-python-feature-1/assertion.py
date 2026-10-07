import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-text-screen-1/8192-python-feature-1/independent/completion.json': '054a7c084f8f7231007f984b2f32024c35f3d8fabd2f1b2810e86c4c7d1898f4', '/workspace/.alt-repair/mimo-text-screen-1/8192-python-feature-1/independent/check.py': '898c129f6a8ed384fbb67ea7d156076570747dc2024efabc0d46541f2bd1691a'}
command=['python3', '/workspace/.alt-repair/mimo-text-screen-1/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_13f5b5b4eb2c41a4b8079046e9367f78'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
