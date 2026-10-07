import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-paths-2/independent/completion.json': '123f43690b45d117478b4291f431bd325e31736d5ecb25a942e509c68e9ea3da', '/home/runner/work/_temp/campaign/8192-python-paths-2/independent/check.py': 'd4f3db99590d15e396855c06f6a99dbd2183d45d9aa5682cf5c3430348b5a6a9'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-paths-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_b6a2028b326d420cb0cdd2c998effc7d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
