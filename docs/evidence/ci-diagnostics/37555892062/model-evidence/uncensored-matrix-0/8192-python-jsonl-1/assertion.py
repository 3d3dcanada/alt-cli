import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-jsonl-1/independent/completion.json': 'fd695fa79259913b89e33eb334c87a347571bf571b0baf3c5f83c5efb05473d8', '/home/runner/work/_temp/campaign/8192-python-jsonl-1/independent/check.py': '7d323b1ad09b42516a71f2d8a0e61f5f8b49436f911cf9c8858bfe6c32799818'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-jsonl-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_8c666f3924964bbd9f2b93f53b8b3357'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
