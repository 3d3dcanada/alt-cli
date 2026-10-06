#!/usr/bin/env python3
"""Bounded real PTY job lifecycle soak. No model weights and no external target."""
import argparse,ctypes,hashlib,json,os,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,default=Path(os.environ.get('ALT_TEST_BINARY','target/debug/alt')));p.add_argument('--rounds',type=int,default=30);p.add_argument('--output',type=Path,required=True);a=p.parse_args();binary=a.binary.resolve();assert 1<=a.rounds<=1000
rows=[];identities=[];boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip()
# Emulate a proper reaping init for this harness's descendants in minimal containers.
assert ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)==0
def reap():
 while True:
  try:
   if os.waitpid(-1,os.WNOHANG)[0]==0:break
  except ChildProcessError:break
def process(identity):
 if not identity:return None
 path=Path('/proc')/str(identity['pid'])
 try:
  fields=(path/'stat').read_text().rsplit(') ',1)[1].split()
  if fields[19]!=identity['start'] or identity['boot']!=boot:return None
  status=dict(line.split(':',1) for line in (path/'status').read_text().splitlines() if ':' in line)
  return {'pid':identity['pid'],'state':fields[0],'rss_bytes':int(status.get('VmRSS','0 kB').split()[0])*1024,'fds':len(list((path/'fd').iterdir()))}
 except (FileNotFoundError,ProcessLookupError):return None
with tempfile.TemporaryDirectory(prefix='alt-soak-') as d:
 root=Path(d)
 def run(*args):
  r=subprocess.run([str(binary),'--data-dir',str(root),'--access','trusted',*args],capture_output=True,text=True,timeout=15,cwd=root);assert r.returncode==0,r.stderr;return json.loads(r.stdout) if r.stdout.lstrip().startswith(('{','[')) else r.stdout
 for n in range(a.rounds):
  started=time.monotonic();ids=[];row={'round':n+1,'passed':False,'live_process_samples':[]}
  try:
   record=run('jobs','start',"printf 'READY\\n'; while read line; do printf 'RECEIPT:%s\\n' \"$line\"; done",'--timeout','30');id=record['id'];ids.append(id)
   identities.extend(record[key] for key in ['supervisor','process'] if record.get(key))
   run('jobs','resize',id,'90','28');run('jobs','input',id,f'round-{n}\n')
   deadline=time.monotonic()+5
   while time.monotonic()<deadline:
    output=run('jobs','logs',id)
    if f'RECEIPT:round-{n}' in str(output):break
    time.sleep(.02)
   else:raise AssertionError('PTY input was lost')
   for key in ['supervisor','process']:
    sample=process(record.get(key));assert sample and sample['state']!='Z','Owned process exited before input confirmation';row['live_process_samples'].append(sample)
    assert sample['fds']<=128 and sample['rss_bytes']<=256*1024*1024,sample
   run('jobs','stop',id);restarted=run('jobs','restart',id);ids.append(restarted['id']);identities.extend(restarted[key] for key in ['supervisor','process'] if restarted.get(key));stopped=run('jobs','stop',restarted['id']);assert stopped['status']!='running';row['passed']=True
  except Exception as error:row['error']=repr(error)
  finally:
   for id in ids:
    try:run('jobs','stop',id)
    except Exception as error:row.setdefault('cleanup_errors',[]).append(repr(error));row['passed']=False
   deadline=time.monotonic()+3
   while True:
    reap();remaining=[sample for identity in identities if (sample:=process(identity))]
    if not remaining or time.monotonic()>=deadline:break
    time.sleep(.02)
   row['remaining_owned_processes']=remaining;row['state_bytes']=sum(f.stat().st_size for f in root.rglob('*') if f.is_file());row['seconds']=time.monotonic()-started
   if remaining or row['state_bytes']>(n+1)*512*1024:row['passed']=False
   rows.append(row)
   a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps({'schema':2,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'rounds':rows,'passed':len(rows)==a.rounds and all(r['passed'] for r in rows),'limits':{'per_process_fds':128,'per_process_rss_bytes':256*1024*1024,'state_bytes_per_round':512*1024,'post_stop_processes':0},'scope':'Actual PTY start/input/resize/stop/restart. Linux subreaper emulates a proper init for test-owned descendants. RSS/FD are point samples, not peaks. Retained job evidence causes expected bounded disk growth. No human usability or model inference measured.'},indent=2)+'\n')
  if not row['passed']:break
 assert all(r['status']!='running' for r in run('jobs','list')),'Owned job leaked'
assert len(rows)==a.rounds and all(r['passed'] for r in rows),'Soak failed; all attempts retained'
print(f'PASS: {len(rows)} PTY lifecycle rounds with bounded process/FD/RSS/disk observations')
