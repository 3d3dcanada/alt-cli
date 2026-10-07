import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-csv-4/independent/completion.json': 'a1d072599e1697a3e0e16351e1d823980e4c9f1ff94bbb3423b1cb71d86e30de', '/home/runner/work/_temp/campaign/8192-python-csv-4/independent/check.py': '0b027e634c81d1622079e5613dd812b40dca2e04d0513c4e02b97f950a199955'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-csv-4/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_4bdd72995c9146e5853a6639168a47c0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
