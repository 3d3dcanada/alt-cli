import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-javascript-config-2/independent/completion.json': '23bbc00f02c6afb45065b242cce4f4cc0567dd318b3df042836f689b83a7f5a5', '/home/runner/work/_temp/campaign/8192-javascript-config-2/independent/check.mjs': '8338cae0fe3a80d24405adf28ddbf77c24c4731853b6cb2dbbeaaad53351d986'}
command=['node', '/home/runner/work/_temp/campaign/8192-javascript-config-2/independent/check.mjs']
receipt='ALT_ORACLE_COMPLETED_e68c85dc6dd84f4fbe32434cdcdff4ac'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
