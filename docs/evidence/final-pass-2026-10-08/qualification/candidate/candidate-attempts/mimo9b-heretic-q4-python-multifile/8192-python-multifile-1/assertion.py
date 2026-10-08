import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-final-qualification/candidate-attempts/mimo9b-heretic-q4-python-multifile/8192-python-multifile-1/independent/completion.json': '5c6b41ad351314d12f96fe306a861f27501f8eb584f87a3dad6d6c984debe32a', '/workspace/.alt-final-qualification/candidate-attempts/mimo9b-heretic-q4-python-multifile/8192-python-multifile-1/independent/check.py': 'f3ba38f8065df4cdd7be0d2238405ee9cce2fc88a368004ad655b785890e367f'}
command=['python3', '/workspace/.alt-final-qualification/candidate-attempts/mimo9b-heretic-q4-python-multifile/8192-python-multifile-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_a9312695baac48c6a84a1deabeee2d81'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
