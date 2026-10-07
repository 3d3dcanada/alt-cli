import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-javascript-config-4/independent/completion.json': 'ef45d615b45cac5146535171ebf7a733b7531ef45a48c9d9f0fed08eebda7ecb', '/home/runner/work/_temp/campaign/8192-javascript-config-4/independent/check.mjs': '62c3dae3846baa78266ae857bfe8fc7b3f42e0d926b11b6291c0866e673a0f10'}
command=['node', '/home/runner/work/_temp/campaign/8192-javascript-config-4/independent/check.mjs']
receipt='ALT_ORACLE_COMPLETED_5103fd5571724d148f8002a8d565ef2a'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
