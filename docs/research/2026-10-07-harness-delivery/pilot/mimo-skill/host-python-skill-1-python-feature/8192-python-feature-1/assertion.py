import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/mimo-skill-pilot/host-python-skill-1-python-feature/8192-python-feature-1/independent/completion.json': '7e9d0082429234227d1818cf4e311a45a93f51bebb232d07ca429ea351a6e6b9', '/workspace/.alt-harness/mimo-skill-pilot/host-python-skill-1-python-feature/8192-python-feature-1/independent/check.py': '6a9c3862cb0c4e4582a19fc66af859784facff2e6f2acbf1dd6f344785bb9ec6'}
command=['python3', '/workspace/.alt-harness/mimo-skill-pilot/host-python-skill-1-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_3328ad37997a458388440ece62095626'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
