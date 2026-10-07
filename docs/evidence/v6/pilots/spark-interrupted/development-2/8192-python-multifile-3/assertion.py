import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-evaluations/v6/spark-full/development-2/8192-python-multifile-3/independent/check.py': 'e791b69b792f4daf6d6bc801292cd2c17d9562a8d1daec415b5e388c3f401b2d'}
command=['python3', '/workspace/.alt-evaluations/v6/spark-full/development-2/8192-python-multifile-3/independent/check.py']
assert all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()), 'Oracle inputs changed'
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1'}
r=subprocess.run(command,env=env,timeout=60)
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if r.returncode==0 else 'failed'}]}))
sys.exit(r.returncode)
