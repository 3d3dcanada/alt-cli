import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-final-sealed/8192-sealed-percent-decoding-1/independent/completion.json': '1b342a38540d15c812f2a69d59bda094c37b6e46b768a7ba3b32c3099f139df6', '/workspace/.alt-repair/mimo-final-sealed/8192-sealed-percent-decoding-1/independent/check.py': '7d3976801ef5009641674e1fe0ddbb2dd970896d9021b49b6e3eb318eaa7741e'}
command=['python3', '/workspace/.alt-repair/mimo-final-sealed/8192-sealed-percent-decoding-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_7fb5020a83d94e349e319074aee44825'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
