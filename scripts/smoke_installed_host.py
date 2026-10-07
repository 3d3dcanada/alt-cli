#!/usr/bin/env python3
"""Install exact preverified package bytes on a container host, with real prior-state recovery."""
import argparse, hashlib, json, os, shutil, subprocess, tempfile
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('package',type=Path);p.add_argument('--previous',type=Path);a=p.parse_args()
repo=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='alt-host-install-') as d:
    root=Path(d);prefix=root/'installed';binary=prefix/'bin/alt';state=root/'state';project=root/'project';project.mkdir()
    (project/'check.py').write_text('assert 6*7==42\n')
    env={**os.environ,'ALT_PREFIX':str(prefix),'ALT_DATA_DIR':str(state)}
    def invoke(*args):
        return subprocess.run([str(binary),'--data-dir',str(state),'--access','trusted',*args],cwd=project,check=True,capture_output=True,text=True)
    if a.previous:
        binary.parent.mkdir(parents=True);shutil.copy2(a.previous/'alt',binary)
        invoke('task','configure-check','host-check','--','python3','check.py');invoke('task','check','host-check')
        invoke('state','backup',str(root/'old-state.tar.gz'))
    installer=['bash',str(a.package/'install.sh')]
    for _ in range(2):subprocess.run(installer,env=env,check=True,capture_output=True)
    assert binary.read_bytes()==(a.package/'alt').read_bytes()
    assert (prefix/'share/doc/alt/SBOM.cdx.json').is_file()
    invoke('task','configure-check','host-check','--','python3','check.py')
    invoke('task','require','behavior','--check','host-check');invoke('task','verify','--run')
    assert json.loads(invoke('task','verify').stdout)['complete']
    subprocess.run(['python3',str(repo/'scripts/smoke_workbench_tui.py')],env={**env,'ALT_TEST_BINARY':str(binary)},cwd=project,check=True)
    if a.previous:
        subprocess.run(installer+['--rollback'],env=env,check=True,capture_output=True)
        assert binary.read_bytes()==(a.previous/'alt').read_bytes()
        state.rename(root/'newer-state-retained')
        invoke('state','restore',str(root/'old-state.tar.gz'),str(state));invoke('task','check','host-check')
        previous_contract=json.loads(invoke('task','export').stdout)['checks'][-1].get('environment',{}).get('verification_contract')
        subprocess.run(installer,env=env,check=True,capture_output=True)
        invoke('task','require','behavior','--check','host-check')
        assert json.loads(invoke('task','status').stdout)['verification']['complete']==(previous_contract=='3')
        invoke('task','verify','--run');assert json.loads(invoke('task','verify').stdout)['complete']
    print(json.dumps({'passed':True,'installed_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'previous_recovery_measured':bool(a.previous),'libc':subprocess.check_output(['ldd','--version'],text=True).splitlines()[0],'scope':'Exact installed binary, repeated install, current checks, actual prior-version state restore, rollback/re-upgrade and real installed TUI'}))
