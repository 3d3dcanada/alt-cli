import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-feature-3/independent/completion.json': '2b38eb7b7096adc0ca1bfbd74eeb9f64a4a0462ce0da8f9c5d24783205cf9e75', '/home/runner/work/_temp/campaign/8192-python-feature-3/independent/check.py': 'd315e929fdcf7528dc9ad58c4c9de17ad2cb075518634b466e20cd341f481166'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-feature-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_3b688078e0c14ee1b1fc36ea8b018bd5'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
