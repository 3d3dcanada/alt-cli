import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-api-2/independent/completion.json': 'd9cda79acfaf5efbc2a8e30579f4060d8de7ce57f46d343ba682975fab61c452', '/home/runner/work/_temp/campaign/8192-python-dependency-api-2/independent/check.py': '43a95151a29be70cd206b1c2dc8ba84b8f5662ac762d9aebf75bfc712937ded5'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-api-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_554963cc0bf445ab939454026420590f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
