import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-migration-5/independent/completion.json': 'b671d0c5965a48739e5ad34858928323a0c099cec53453c6a2fe481fa2a0de13', '/home/runner/work/_temp/campaign/8192-python-migration-5/independent/check.py': '254f8997e394caa9f5222dda7e90238889ccebead2a573f170165abefd0c1042'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-migration-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_6efaba8d82f442a58464ca821302d333'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
