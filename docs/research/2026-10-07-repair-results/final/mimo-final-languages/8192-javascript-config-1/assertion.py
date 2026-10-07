import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-final-languages/8192-javascript-config-1/independent/completion.json': '46b6b7338a2bb789a0a518a975ad835d4e3d9450d9a9e0c47375e8f34088ab8b', '/workspace/.alt-repair/mimo-final-languages/8192-javascript-config-1/independent/check.mjs': '191c0afb75ef9187172e51c9a1ebca14975ea589dc2f8020d9201685ea89ff02'}
command=['node', '/workspace/.alt-repair/mimo-final-languages/8192-javascript-config-1/independent/check.mjs']
receipt='ALT_ORACLE_COMPLETED_a274544f3a364a2a8d34d3c5a3e4f552'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
