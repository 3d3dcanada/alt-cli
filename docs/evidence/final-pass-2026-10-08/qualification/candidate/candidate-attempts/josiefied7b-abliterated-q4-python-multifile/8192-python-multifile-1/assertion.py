import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-final-qualification/candidate-attempts/josiefied7b-abliterated-q4-python-multifile/8192-python-multifile-1/independent/completion.json': '5b7f82be577233453594d3eb87d4f64f1e796594eddfee13b811fd31139c0b4e', '/workspace/.alt-final-qualification/candidate-attempts/josiefied7b-abliterated-q4-python-multifile/8192-python-multifile-1/independent/check.py': 'eb5793d1feec529529a6095580394fc1d4e0be416d789558458a9d402d24937b'}
command=['python3', '/workspace/.alt-final-qualification/candidate-attempts/josiefied7b-abliterated-q4-python-multifile/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_585628ad668d4a2d953faf5eb48705e5'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
