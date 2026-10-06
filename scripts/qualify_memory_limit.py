#!/usr/bin/env python3
"""Exercise real Linux memory limits without changing the selected uncensored model."""
import argparse, hashlib, json, os, subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',type=Path,required=True);p.add_argument('--runtime',type=Path,required=True)
p.add_argument('--model',type=Path,required=True);p.add_argument('--sha256',required=True)
p.add_argument('--uncensored',action='store_true',required=True);p.add_argument('--output',type=Path,required=True)
a=p.parse_args();root=a.output.resolve();root.mkdir(parents=True,exist_ok=False)
binary=a.binary.resolve();model=a.model.resolve();runtime=a.runtime.resolve()
h=hashlib.sha256()
with model.open('rb') as f:
    for part in iter(lambda:f.read(8*1024*1024),b''):h.update(part)
assert h.hexdigest()==a.sha256,'Wrong model artifact'
state=root/'state';state.mkdir();project=root/'project';project.mkdir()
(state/'preferences.toml').write_text('project='+json.dumps(str(project))+'\nruntime_path='+json.dumps(str(runtime))+'\n[runtime]\ngpu_layers=0\nthreads=2\nbatch=128\n')
setup=subprocess.run([str(binary),'--data-dir',str(state),'models','import',str(model),'--uncensored','--use'],capture_output=True,text=True,check=True)
(root/'setup.stdout').write_text(setup.stdout);(root/'setup.stderr').write_text(setup.stderr)
before=(state/'config.toml').read_bytes()
image='ubuntu:24.04';prefix=['docker','run','--rm','--network','none','--memory','512m','--memory-swap','512m','--cpus','2']
for source in [root,binary,model,runtime.parent]:
    prefix+=['-v',str(source)+':'+str(source)+('' if source==root else ':ro')]
prefix += ['-w',str(project),image,str(binary),'--data-dir',str(state)]
hardware=subprocess.run(prefix+['hardware'],capture_output=True,text=True,check=True,timeout=30)
host=json.loads(hardware.stdout);assert host['ram']<=512*1024*1024,host
result=subprocess.run(prefix+['qualify','--contexts','2048','--repeats','1'],capture_output=True,text=True,timeout=120)
(root/'command.stdout').write_text(result.stdout);(root/'command.stderr').write_text(result.stderr)
report=json.loads(result.stdout);errors=[row['measurement'].get('error','') for row in report['rows']]
unchanged=(state/'config.toml').read_bytes()==before
passed=result.returncode!=0 and unchanged and errors and all('Available RAM is below' in e for e in errors)
summary={'passed':bool(passed),'expected':'Recorded insufficient-RAM failure without selecting a substitute model','command':prefix+['qualify','--contexts','2048','--repeats','1'],'exit':result.returncode,'selected_profile_unchanged':unchanged,'errors':errors,'hardware':host,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'model_sha256':a.sha256,'image_id':subprocess.check_output(['docker','image','inspect',image,'--format','{{.Id}}'],text=True).strip()}
(root/'invocation.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({'passed':bool(passed),'errors':errors,'selected_profile_unchanged':unchanged}))
raise SystemExit(0 if passed else 1)
