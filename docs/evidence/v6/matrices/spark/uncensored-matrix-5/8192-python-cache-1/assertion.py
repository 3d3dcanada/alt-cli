import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cache-1/independent/completion.json': 'b3f6b55fa889dab941ffd8dc6f5d312c48eaaa463a30f79933b3fdcd4145039a', '/home/runner/work/_temp/campaign/8192-python-cache-1/independent/check.py': '63c0b9b1b6c958999621d5e2db04163ffa2efcb5570fa8e94bf9424b82826ec7'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cache-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e233811b48724e7f9dff9e7c19aba39d'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
