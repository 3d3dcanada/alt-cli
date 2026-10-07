#!/usr/bin/env python3
"""Verify Rust snapshot isolation, pinned inputs and collection after real builds."""
import hashlib, json, os, shutil, subprocess, sys, tempfile
from pathlib import Path
from acceptance_projects import CASES, setup, check, write, oracle_inputs, assertion_script

repo=Path(__file__).resolve().parents[1]
binary=Path(os.environ.get('ALT_TEST_BINARY',repo/'target/debug/alt')).resolve()
with tempfile.TemporaryDirectory(prefix='alt-oracle-collection-') as d:
    root=Path(d); campaign=root/'campaign'; campaign.mkdir(); run=campaign/'rust-probe'; run.mkdir()
    project, command=setup('rust-feature',run)
    inputs=oracle_inputs(run)
    assert not check(project,command)['passed']
    snapshot=root/'snapshot'; shutil.copytree(project,snapshot); write(snapshot,CASES['rust-feature']['fixed'])
    assert check(snapshot,command)['passed'], 'Oracle must evaluate the selected snapshot instead of its original project'
    assert not check(project,command)['passed'], 'Original broken source must remain broken'
    assert inputs==oracle_inputs(run), 'Running the oracle modified its protected source inputs'
    assertion=run/'assertion.py'; assertion.write_text(assertion_script(run,command,inputs))
    base=[str(binary),'--data-dir',str(root/'state'),'--access','trusted']
    for args in [
        ['task','configure-check','acceptance','--','python3',str(assertion)],
        ['task','contract','acceptance','--kind','tests','--format','json','--report','.alt-acceptance.json','--assertion',str(assertion)],
        ['task','require','behavior','--check','acceptance']]:
        subprocess.run(base+args,cwd=snapshot,check=True,capture_output=True)
    for _ in range(2):
        r=subprocess.run(base+['task','verify','--run'],cwd=snapshot,capture_output=True,text=True)
        assert r.returncode==0, r.stderr
        status=json.loads(subprocess.check_output(base+['task','verify'],cwd=snapshot,text=True))
        assert status['behavioral_acceptance'] and status['complete']
    # Reproduce the collector's observed failure trigger without bundling binaries.
    generated=run/'independent/target/debug/generated-binary'; generated.parent.mkdir(parents=True)
    with generated.open('wb') as f: f.truncate(6*1024*1024)
    (run/'fixture.json').write_text(json.dumps({'oracle_sha256_before':inputs}))
    (run/'report.json').write_text(json.dumps({'scope':'Deterministic harness validation; no model weights','oracle_unchanged':True}))
    (run/'turn.jsonl').write_text('')
    destination=root/'retained'
    subprocess.run([sys.executable,str(repo/'scripts/collect_live_evidence.py'),str(campaign),str(destination)],check=True)
    manifest=json.loads((destination/'RAW-SHA256.json').read_text())
    assert all('/target/' not in name for name in manifest)
    assert all(hashlib.sha256((destination/name).read_bytes()).hexdigest()==digest for name,digest in manifest.items())
    for name in inputs: assert 'rust-probe/independent/'+name in manifest
    protected=run/'independent/src/lib.rs'; protected.write_text(protected.read_text()+'\n// Changed oracle input\n')
    rejected=subprocess.run(base+['task','verify','--run'],cwd=snapshot,capture_output=True,text=True)
    assert rejected.returncode!=0, 'Changed independent assertion inputs must invalidate verification'
    python_run=root/'early-exit'; python_project,python_command=setup('python-feature',python_run)
    python_inputs=oracle_inputs(python_run)
    (python_project/'formatter.py').write_text('import sys\nsys.exit(0)\n')
    python_assertion=python_run/'assertion.py'; python_assertion.write_text(assertion_script(python_run,python_command,python_inputs))
    for args in [
        ['task','configure-check','acceptance','--','python3',str(python_assertion)],
        ['task','contract','acceptance','--kind','tests','--format','json','--report','.alt-acceptance.json','--assertion',str(python_assertion)],
        ['task','require','behavior','--check','acceptance']]:
        subprocess.run(base+args,cwd=python_project,check=True,capture_output=True)
    early=subprocess.run(base+['task','verify','--run'],cwd=python_project,capture_output=True,text=True)
    assert early.returncode!=0, 'Zero exit before assertion completion must not produce a passing structured check'
    print(json.dumps({'passed':True,'snapshot_repair_accepted':True,'original_defect_rejected':True,'repeated_pinned_checks':2,'generated_outputs_excluded':True,'changed_oracle_rejected':True,'early_zero_exit_rejected_by_structured_check':True,'model_weights_used':False}))
