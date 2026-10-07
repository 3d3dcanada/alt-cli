import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-final-tui/8192-python-multifile-1/independent/completion.json': '219f899128ef562ccbcddf8a64b1f17edc1a3fbc2c598ffe3073005f452333bd', '/workspace/.alt-repair/mimo-final-tui/8192-python-multifile-1/independent/check.py': '677fed3926c8a4ecd9d020d5b33a88526097724401d052e9c283e0204e6fd022'}
command=['python3', '/workspace/.alt-repair/mimo-final-tui/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a68b9f3fa5044fc3ae52074658727f15'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
