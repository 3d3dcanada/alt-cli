import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-final-sealed/8192-sealed-intervals-1/independent/completion.json': 'c6e499e82df3337f0d942da2341207497d6761f0cb49762d36315d972f6951f9', '/workspace/.alt-repair/mimo-final-sealed/8192-sealed-intervals-1/independent/check.py': '4f43c8d1c07cd79109e908ec2da574c27c2b6f5756634b086b974042478627f0'}
command=['python3', '/workspace/.alt-repair/mimo-final-sealed/8192-sealed-intervals-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_f7a54cc9cecc4c8e830102e038e7b3bb'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
