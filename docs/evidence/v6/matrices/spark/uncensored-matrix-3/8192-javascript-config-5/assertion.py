import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-javascript-config-5/independent/completion.json': '120e3f94ecaf4e799ab20ed2af3517c004768afc8ccdac30c4f5097fbea5cea2', '/home/runner/work/_temp/campaign/8192-javascript-config-5/independent/check.mjs': '431718a70a85c34619781e1b222ebbcf10a748088733754f0b768c0cbebea567'}
command=['node', '/home/runner/work/_temp/campaign/8192-javascript-config-5/independent/check.mjs']
receipt='ALT_ORACLE_COMPLETED_c45c3b438f774e80b77b5c12e8085fb3'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
