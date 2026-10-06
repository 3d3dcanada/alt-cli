#!/usr/bin/env python3
"""Actual rust-analyzer references and checkpointed rename against a tiny crate."""
import argparse,json,subprocess,tempfile
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--alt',type=Path,default=Path('target/debug/alt'));p.add_argument('--server',default='rust-analyzer');a=p.parse_args();binary=a.alt.resolve()
with tempfile.TemporaryDirectory(prefix='alt-real-lsp-')as t:
 root=Path(t);project=root/'project';(project/'src').mkdir(parents=True);(project/'Cargo.toml').write_text('[package]\nname="lsp_fixture"\nversion="0.1.0"\nedition="2021"\n');source=project/'src/lib.rs';source.write_text('pub fn old_name() -> i32 { 42 }\npub fn caller() -> i32 { old_name() }\n')
 base=[str(binary),'--data-dir',str(root/'state'),'--access','trusted','language','--server',a.server,'src/lib.rs','--line','1','--column','8']
 r=subprocess.run(base,cwd=project,capture_output=True,text=True,timeout=90);assert r.returncode==0,r.stderr;refs=json.loads(r.stdout)['references'];assert len(refs)>=2,refs
 r=subprocess.run(base+['--rename','new_name','--apply'],cwd=project,capture_output=True,text=True,timeout=90);assert r.returncode==0,r.stderr;assert source.read_text().count('new_name')==2;assert 'old_name' not in source.read_text();subprocess.run(['cargo','check','--offline'],cwd=project,check=True,capture_output=True)
 print('PASS: actual rust-analyzer references, reviewed rename changes, checkpoint application and independent cargo check')
