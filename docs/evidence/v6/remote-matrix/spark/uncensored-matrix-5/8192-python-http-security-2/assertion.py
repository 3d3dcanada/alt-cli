import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-http-security-2/independent/completion.json': '5bf9d36d5404ee9b9ac28169adb7992fc4618be2438a8ea345a0b8ebad3c5245', '/home/runner/work/_temp/campaign/8192-python-http-security-2/independent/check.py': '0fd9bd9ae79275b3571def5813e61552eae3df714debbac31826adcaed575697'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-http-security-2/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_90744bffda3d4619b72debed7347d386'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
