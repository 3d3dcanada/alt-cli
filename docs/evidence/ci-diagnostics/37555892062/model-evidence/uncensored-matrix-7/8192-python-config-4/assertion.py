import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-config-4/independent/completion.json': '0a2654ddc748697bbd74af930b8f27e1cdb9874c89b896002dcc73abdb39bfdc', '/home/runner/work/_temp/campaign/8192-python-config-4/independent/check.py': '97a02f182d6ba7e6b4271eb577862404bd40ea7704c0b82c3368e8c71592e376'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-config-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a615775322d04c75aa694c7a954c8519'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
