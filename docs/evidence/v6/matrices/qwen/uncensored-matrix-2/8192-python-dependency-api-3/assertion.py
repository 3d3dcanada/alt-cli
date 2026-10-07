import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-dependency-api-3/independent/completion.json': '1fd6c39f07a501f42d42624207ca6e62ce27aa54818fc874108136cd01c2dcbc', '/home/runner/work/_temp/campaign/8192-python-dependency-api-3/independent/check.py': 'e076a81fb74343cb6252d3be30a9d7179d8eb5b2442eeada3d23727b76ac21fd'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-dependency-api-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_7ba7888d7e254ee79f6bcd131d0c3a4c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
