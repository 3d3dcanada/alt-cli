import json,os,shutil,subprocess,sys,tempfile
from pathlib import Path
source=Path(__file__).parent/'src/lib.rs'
with tempfile.TemporaryDirectory(prefix='alt-rust-oracle-') as d:
 root=Path(d); (root/'project').symlink_to(Path.cwd(),target_is_directory=True)
 crate=root/'independent'; (crate/'src').mkdir(parents=True)
 shutil.copy2(source,crate/'src/lib.rs')
 (crate/'Cargo.toml').write_text('[package]\nname="independent_oracle"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nfixture_stats={path='+json.dumps(str(Path.cwd()))+'}\n')
 env={**os.environ,'CARGO_TARGET_DIR':str(root/'target')}
 result=subprocess.run(['cargo','test','--offline','--manifest-path',str(crate/'Cargo.toml')],env=env,capture_output=True,text=True)
 print(result.stdout,end=''); print(result.stderr,end='',file=sys.stderr)
 passed=result.returncode==0 and 'test behavior ... ok' in result.stdout and 'test result: ok. 1 passed; 0 failed;' in result.stdout
 if passed: print('ALT_ORACLE_COMPLETED_4b13d211d4b547468d22e79bdc242d64')
 sys.exit(0 if passed else 1)
