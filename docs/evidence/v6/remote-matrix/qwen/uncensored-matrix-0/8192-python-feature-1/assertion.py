import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-feature-1/independent/completion.json': 'e9ac9f81fa9bd98dfb885d82c00b77b5cc5a94c2460e6ad5eb3cf61c7e9fa114', '/home/runner/work/_temp/campaign/8192-python-feature-1/independent/check.py': '73290cdd91e319dcd4ba31e05694e2d65f53b111478b480df91b4b1b6369b456'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_43111a2f67794ea99044fb1bddb2695a'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
