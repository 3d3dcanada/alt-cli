import hashlib,json,os,subprocess,sys
from pathlib import Path
hashes={'/workspace/.alt-evaluations/v6/spark-full/development-1/8192-rust-feature-5/independent/Cargo.lock': 'bd9c6c41ac04d765aea97cd5b9a43a6d61bc9a1984dd92cda078dfbc08b201f0', '/workspace/.alt-evaluations/v6/spark-full/development-1/8192-rust-feature-5/independent/Cargo.toml': '1f41cd27a35a6597831e1ea12089ddebf8003ddc57960b3cfd3ecf00b7b15ab9', '/workspace/.alt-evaluations/v6/spark-full/development-1/8192-rust-feature-5/independent/src/lib.rs': '942eeda306422d082f001ee4a730b80bbc753cf5f1f67d2e3f16fe938be4ca5a'}
command=['cargo', 'test', '--offline', '--manifest-path', '/workspace/.alt-evaluations/v6/spark-full/development-1/8192-rust-feature-5/independent/Cargo.toml']
assert all(Path(p).is_file() and hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in hashes.items()), 'Oracle inputs changed'
env={**os.environ,'ALT_PROJECT_URL':Path.cwd().as_uri(),'PYTHONDONTWRITEBYTECODE':'1'}
r=subprocess.run(command,env=env,timeout=60)
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'requested behavior assertions','status':'passed' if r.returncode==0 else 'failed'}]}))
sys.exit(r.returncode)
