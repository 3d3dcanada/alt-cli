import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cli-5/independent/completion.json': 'a4b4e2e81988ee1c7b6a0db354e04d174a6dcb53f3eb20826923119f6c132264', '/home/runner/work/_temp/campaign/8192-python-cli-5/independent/check.py': '6ced25802105218b2fad4429bcdd96fd7d49376fe65621435faabd5acaff497f'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cli-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_dada7055389a4df1bfdcb154a9692bbb'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
