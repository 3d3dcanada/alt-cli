import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-http-security-3/independent/completion.json': 'a627928213b55fd1a8f243b33fe8cb027013d3757e114398c09acd0435d9fd26', '/home/runner/work/_temp/campaign/8192-python-http-security-3/independent/check.py': '6000c2dc17649896617664b0f39316ca2d88a9c3488cb508a9573f8e1d28e8eb'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-http-security-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_e28758d56f9d474e899857ad58dfd32a'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
