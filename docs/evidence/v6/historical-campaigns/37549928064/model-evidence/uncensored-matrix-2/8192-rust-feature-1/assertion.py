import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/home/runner/work/_temp/campaign/8192-rust-feature-1/independent/Cargo.toml': '3896be319ec8e6e9099a67e5db3e98debb76a2ce6e9874bd283c680522b32418', '/home/runner/work/_temp/campaign/8192-rust-feature-1/independent/Cargo.lock': 'bd9c6c41ac04d765aea97cd5b9a43a6d61bc9a1984dd92cda078dfbc08b201f0', '/home/runner/work/_temp/campaign/8192-rust-feature-1/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['cargo', 'test', '--offline', '--manifest-path', '/home/runner/work/_temp/campaign/8192-rust-feature-1/independent/Cargo.toml']
assert all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()), 'Oracle inputs changed'
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1'}
r=subprocess.run(command,env=env,timeout=60)
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if r.returncode==0 else 'failed'}]}))
sys.exit(r.returncode)
