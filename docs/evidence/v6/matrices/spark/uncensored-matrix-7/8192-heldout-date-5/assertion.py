import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-heldout-date-5/independent/completion.json': '860786c4e8e066bf274f159bcb21deb4e3b8062f93e166011d394f64ef3defaf', '/home/runner/work/_temp/campaign/8192-heldout-date-5/independent/check.py': '0540788cf22a7ca48e310a464ba988f6d82fdfd4ab3b044a0cee61be36f52afb'}
command=['python3', '/home/runner/work/_temp/campaign/8192-heldout-date-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_bebc6538807a43a6a449342c722bde79'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
