import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-evaluations/v6/spark-full/development-2/8192-javascript-config-5/independent/check.mjs': '5128bd834e6ea5be4e0dbae6a2bdc7c1a88cfc198fba78b5f6d0e6667ca71290'}
command=['node', '/workspace/.alt-evaluations/v6/spark-full/development-2/8192-javascript-config-5/independent/check.mjs']
assert all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()), 'Oracle inputs changed'
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1'}
r=subprocess.run(command,env=env,timeout=60)
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if r.returncode==0 else 'failed'}]}))
sys.exit(r.returncode)
