import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-service-config-1/independent/completion.json': 'f12b2e7248c78bcfb99fc08be2cb27feaa2fd326dece01103cd4e251090bb6df', '/home/runner/work/_temp/campaign/8192-python-service-config-1/independent/check.py': '22142405d713b9fb13b5b5f7f64cfd577656879362e8b65303b5ba519ba6ec7e'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-service-config-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_bfb16513e93b431fb7d0dbd55fa33875'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
