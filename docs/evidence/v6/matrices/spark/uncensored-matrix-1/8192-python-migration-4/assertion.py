import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-migration-4/independent/completion.json': '1950d343f44951de9c67e47d5732912001413890ad97ed2de52a7dcddfc96047', '/home/runner/work/_temp/campaign/8192-python-migration-4/independent/check.py': '5283e1a7fdc4dbe9c342328757a6dedc38de76ec90be103ab406231eda762836'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-migration-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c8edaaeb1fec431984eeef8a1099d14e'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
