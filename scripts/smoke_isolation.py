#!/usr/bin/env python3
"""Exercise the actual guided check boundary where Linux namespaces are enabled."""
import argparse,json,os,subprocess,tempfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--alt',type=Path,required=True);a=p.parse_args();binary=str(a.alt.resolve())
with tempfile.TemporaryDirectory(prefix='alt-isolation-') as t:
 root=Path(t);project=root/'project';project.mkdir();state=root/'state';secret=root/'host-secret';secret.write_text('DO_NOT_EXPOSE')
 probe=f'''import socket
from pathlib import Path
assert not Path({str(secret)!r}).exists(), 'Host file leaked into sandbox'
s = socket.socket()
s.settimeout(1)
try:
    s.connect(('1.1.1.1', 443))
except OSError:
    pass
else:
    raise AssertionError('Network was available')
finally:
    s.close()
Path('probe.py').write_text('changed only inside the source copy')
print('ALT_ISOLATION_OK')
'''
 (project/'probe.py').write_text(probe)
 base=[binary,'--data-dir',str(state),'--access','guided']
 subprocess.run(base+['task','configure-check','isolation','--','python3','probe.py'],cwd=project,check=True,capture_output=True)
 r=subprocess.run(base+['task','check','isolation'],cwd=project,capture_output=True,text=True,timeout=30)
 result=json.loads(r.stdout)
 assert r.returncode==0,result
 assert result['exit_code']==0 and result['error'] is None and 'ALT_ISOLATION_OK' in result['output'],result
 assert (project/'probe.py').read_text()==probe and secret.read_text()=='DO_NOT_EXPOSE'
 print(json.dumps({'passed':True,'boundary':result['isolation'],'source_unchanged':True,'host_file_hidden':True,'network_blocked':True}))
