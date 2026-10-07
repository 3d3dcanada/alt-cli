import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-repair/mimo-final-sealed/8192-sealed-stream-lines-1/independent/completion.json': 'c13af6c11653dc7abdf2d1a588dcbfa7e6ad5f533db3f67c780a22480ec37d80', '/workspace/.alt-repair/mimo-final-sealed/8192-sealed-stream-lines-1/independent/check.py': '1ad8e450ea6bc3d2826e6f2453253f0714512c4e0bf600c35d683ae90d7ed59c'}
command=['python3', '/workspace/.alt-repair/mimo-final-sealed/8192-sealed-stream-lines-1/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_23e5344bc75f421d9289bbb869e8d32c'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
