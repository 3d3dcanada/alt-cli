import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-csv-3/independent/completion.json': '4afed62e849f093505ba3363eea73caebfc419d843b07006084b98e79a979995', '/home/runner/work/_temp/campaign/8192-python-csv-3/independent/check.py': 'cd0081cacd4aa421ced80d096628711c0a5a6512ad8f177f2d1b5f14f238a82a'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-csv-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e3416980edaf4f54b65f0356b28f8db0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
