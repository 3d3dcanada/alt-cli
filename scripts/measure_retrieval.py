#!/usr/bin/env python3
"""Index beyond check-snapshot file limits; verify incremental deletion and edit."""
import argparse,json,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--alt',type=Path,default=Path('target/release/alt'));p.add_argument('--output',type=Path);a=p.parse_args();binary=a.alt.resolve()
with tempfile.TemporaryDirectory(prefix='alt-retrieval-')as t:
 root=Path(t);project=root/'project';project.mkdir();state=root/'state';count=5001
 for n in range(count):(project/f'module_{n}.py').write_text(f'def feature_{n}(value):\n    """Cedar module {n}."""\n    return value + {n}\n')
 base=[str(binary),'--data-dir',str(state),'task'];measurements=[]
 def index(label):
  start=time.monotonic();r=subprocess.run(base+['index'],cwd=project,capture_output=True,text=True,check=True);v=json.loads(r.stdout);measurements.append({'phase':label,'seconds':round(time.monotonic()-start,3),'report':v});return v
 assert index('initial')['indexed']==count;assert index('unchanged')['updated']==0
 (project/'module_0.py').write_text('def replacement_symbol(): return 1\n');(project/'module_1.py').unlink();v=index('edit_and_delete');assert v['updated']==1 and v['removed']==1
 report={'files':count,'machine':'CPU cloud, filesystem cache warm after file creation','measurements':measurements,'scope':'Index discovery exceeds 4096-file editing/check-snapshot limit; those mutation limits remain explicit.'}
 if a.output:a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report,indent=2))
