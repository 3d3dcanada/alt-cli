import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cache-3/independent/completion.json': 'f18a2ab65088f15376462759d5b10ec8c4a26894c2f7f56f7dcad91131345d78', '/home/runner/work/_temp/campaign/8192-python-cache-3/independent/check.py': 'aa7786068d2cbacf6222b2f41a3c3c15f4a58d58a6e54c5be87364049da34c35'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cache-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_f31fd60600cb436aa236f5b48938d3f4'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
