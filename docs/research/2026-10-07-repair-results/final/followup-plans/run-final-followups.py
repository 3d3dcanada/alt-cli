import hashlib,json,subprocess,sys
from pathlib import Path
repo=Path('/workspace/alt-cli'); root=Path('/workspace/.alt-repair')
binary=repo/'target/portable-glibc231/release/alt'
engine=Path('/workspace/.alt-tools/goose')
runtime=Path('/workspace/.alt-tools/llama-b11429/llama-b11429/llama-server')
small=json.loads((root/'small-scalar-followup-plan.json').read_text())
reasoning=json.loads((root/'rust-followup-plan.json').read_text())
build=json.loads(subprocess.check_output([str(binary),'build-info']))
assert build['source_sha256']==small['source_sha256']
# The first sequence must finish before another model loads.
assert 'END josiefied-final exit' in (root/'final-live.log').read_text()
assert sum(len(list((root/name).glob('*/report.json'))) for name in ['mimo-final-feature','mimo-final-languages','mimo-final-sealed','mimo-final-tui','josiefied-final'])==11
common=[sys.executable,str(repo/'scripts/live_acceptance.py'),'--suite','v5','--binary',str(binary),'--engine',str(engine),'--runtime',str(runtime),'--uncensored','--verification-plan','--partition','development','--contexts','8192','--repeats','1','--workflow','host','--generated-tokens','8192','--requests','12','--threads','2','--batch','128','--temperature','0']
trials=[{'name':'mimo-rust-followup','model':'/tmp/alt-native-requalification/MiMo-V2.6-Distill-Qwen-9B-heretic.Q4_K_M.gguf','sha256':'00877ff79174b79f72c5ec704901fc958400a0b0d390fb61a068281618116dd1','case':'rust-feature','output_tokens':reasoning['output_tokens'],'reasoning_tokens':reasoning['reasoning_tokens'],'timeout_seconds':reasoning['timeout_seconds'],'skill':reasoning['active_skill']},*[{**r,'case':small['case'],'output_tokens':small['output_tokens'],'timeout_seconds':small['timeout_seconds']} for r in small['scheduled']]]
trials.append(json.loads((root/'josiefied-lines-followup-plan.json').read_text())['trial'])
results=[]
for trial in trials:
    name=trial['name']; destination=root/name
    cmd=common+['--tool-profile',trial.get('tool_profile','compact'),'--model',trial['model'],'--sha256',trial['sha256'],'--cases',trial['case'],'--output-tokens',str(trial['output_tokens']),'--timeout',str(trial['timeout_seconds']),'--output',str(destination)]
    if trial.get('reasoning_tokens') is not None: cmd+=['--reasoning-tokens',str(trial['reasoning_tokens'])]
    if trial.get('skill'): cmd+=['--skill',trial['skill']]
    print('START',name,flush=True)
    with (root/(name+'.log')).open('w') as log:
        run=subprocess.run(cmd,cwd=repo,stdout=log,stderr=subprocess.STDOUT)
    if (destination/'summary.json').is_file():
        summary=json.loads((destination/'summary.json').read_text())
        (destination/'BUILD.json').write_text(json.dumps(build,indent=2)+'\n')
        assert summary['total']==1
        assert summary['configuration']['binary_sha256']==hashlib.sha256(binary.read_bytes()).hexdigest()
        results.append({'name':name,'exit':run.returncode,'passed':summary['passed'],'total':summary['total']})
        print('END',name,'exit',run.returncode,'passed',summary['passed'],'/',summary['total'],flush=True)
    else:
        results.append({'name':name,'exit':run.returncode,'error':'No completed report; retain raw failure log','passed':0,'total':0})
        print('ERROR',name,'exit',run.returncode,flush=True)
(root/'followups-summary.json').write_text(json.dumps({'trials':results,'policy':'All scheduled development follow-ups retained; no held-out tuning or default promotion'},indent=2)+'\n')
sys.exit(0 if all(r['passed']==r['total']==1 for r in results) else 1)
