import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-harness/mimo-skill-pilot/host-no-skill-1-python-feature/8192-python-feature-1/independent/completion.json': 'f3f12c968387a37cf6cdb2a8a38175b263322459df88b3b138d3825b761f19d4', '/workspace/.alt-harness/mimo-skill-pilot/host-no-skill-1-python-feature/8192-python-feature-1/independent/check.py': '44831b3b40e1b4ba8c04e0ed63affeb64e3a44d6555f4fc071971e728d84e8ed'}
command=['python3', '/workspace/.alt-harness/mimo-skill-pilot/host-no-skill-1-python-feature/8192-python-feature-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4efdc0124dce49dfaa3a949d1d3b354a'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
