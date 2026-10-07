import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-javascript-config-2/independent/completion.json': 'eabdc4e185b1eff1951223e745605d2c931ef912f93d38cdb3ecba7f573ab419', '/home/runner/work/_temp/campaign/8192-javascript-config-2/independent/check.mjs': '06e85ae75298608e8dc93ca53bc211f81420e827c6f58f6b4e087e2310723e9d'}
command=['node', '/home/runner/work/_temp/campaign/8192-javascript-config-2/independent/check.mjs']
receipt='ALT_ORACLE_COMPLETED_4bc58fa14ba84781bd13e16c9ac3725f'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
