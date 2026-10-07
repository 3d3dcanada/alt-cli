import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-python-cache-5/independent/completion.json': '56231ab1f66e0299e6d812f4f5dd28021629b1777a76194b4f4e82a37fae9948', '/home/runner/work/_temp/campaign/8192-python-cache-5/independent/check.py': '37b32aacae9fcf31a7323db5d6b2d53e765762eaaa995fc3d6a2d2800f53f835'}
command=['python3', '/home/runner/work/_temp/campaign/8192-python-cache-5/independent/check.py']
receipt='ALT_ORACLE_COMPLETED_c02dc4fb11b24bfab1c04c0ebcc851d0'
if not all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()): raise RuntimeError('Oracle inputs changed')
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1','PYTHONOPTIMIZE':'0'}
r=subprocess.run(command,env=env,timeout=60,capture_output=True,text=True)
print(r.stdout,end='');print(r.stderr,end='',file=sys.stderr)
passed=r.returncode==0 and receipt in r.stdout.splitlines()
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if passed else 'failed'}]}))
sys.exit(0 if passed else 1)
