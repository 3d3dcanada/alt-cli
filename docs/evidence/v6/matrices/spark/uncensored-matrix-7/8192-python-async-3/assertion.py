import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-async-3/independent/completion.json': 'f29f37df17a7ec73b4bb9c30abde5021740e870aca912c5511e10814ced1e81a', '/home/runner/work/_temp/campaign/8192-python-async-3/independent/check.py': 'db73c0717ebade23e86bf40abe9bf54bbf622e222ddc34748c512abb22818612'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-async-3/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_cfbcd0c13f644758b1577a8fec4fe5aa'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
