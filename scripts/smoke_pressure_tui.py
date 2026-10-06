#!/usr/bin/env python3
"""Thirty keyboard journeys with project-lock pressure; measures redraw and cancellation independently."""
import argparse,fcntl,hashlib,json,os,pty,select,struct,subprocess,tempfile,termios,time,statistics
from pathlib import Path
import pyte
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,default=Path(os.environ.get('ALT_TEST_BINARY','target/debug/alt')));p.add_argument('--rounds',type=int,default=30);p.add_argument('--output',type=Path,required=True);a=p.parse_args();binary=a.binary.resolve();assert 1<=a.rounds<=1000
rows=[]
for n in range(a.rounds):
 with tempfile.TemporaryDirectory(prefix='alt-pressure-') as d:
  root=Path(d);state=root/'state';state.mkdir();project=root/'project';project.mkdir()
  for i in range(128):(project/f'file-{i}.txt').write_text('Cedar 🙂 line\n'*100)
  (state/'preferences.toml').write_text('project='+json.dumps(str(project))+'\n')
  subprocess.run([str(binary),'--data-dir',str(state),'task','checks'],cwd=project,check=True,capture_output=True)
  lock=next((state/'projects').glob('*/project.lock')).open('r+');fcntl.flock(lock,fcntl.LOCK_EX)
  master,slave=pty.openpty();width,height=[(120,40),(80,24),(60,18)][n%3];fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',height,width,0,0));original=termios.tcgetattr(slave)
  proc=subprocess.Popen([str(binary),'--data-dir',str(state)],stdin=slave,stdout=slave,stderr=slave,cwd=project,env={**os.environ,'TERM':'xterm-256color'})
  screen=pyte.Screen(width,height);stream=pyte.ByteStream(screen)
  def wait(text,timeout=5):
   started=time.monotonic()
   while time.monotonic()-started<timeout:
    if select.select([master],[],[],.01)[0]:stream.feed(os.read(master,65536))
    if text in '\n'.join(screen.display):return (time.monotonic()-started)*1000
   raise AssertionError('Missing '+text+'\n'+'\n'.join(screen.display))
  def send(value):os.write(master,value)
  row={'round':n+1,'size':[width,height],'passed':False}
  try:
   wait('ALT');send(b'\x1b9');time.sleep(.08) # queued file read is deliberately blocked by flock
   start=time.monotonic();send(b'\x1b2');draft=f'DRAFT-{n}-caf';send(b'\x1b[200~'+(draft+'é 🙂').encode()+b'\x1b[201~');wait(draft);row['input_to_visible_ms']=(time.monotonic()-start)*1000
   start=time.monotonic();send(b'\x1b');wait('cancelled',timeout=3);row['cancel_to_visible_ms']=(time.monotonic()-start)*1000
   send(b'\r');wait(draft)
   fcntl.flock(lock,fcntl.LOCK_UN);send(b'\x1b9');wait('file-');send(b'\x1b2');wait(draft)
   send(b'\x11');proc.wait(timeout=10);assert proc.returncode==0 and termios.tcgetattr(slave)==original
   row['passed']=True
  except Exception as error:
   row['error']=str(error)
  finally:
   if proc.poll() is None:proc.terminate();proc.wait(timeout=10)
   lock.close();os.close(master);os.close(slave)
  rows.append(row)
  report={'schema':1,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'journeys':rows,'passed':sum(r['passed'] for r in rows),'scope':'Automated keyboard/Unicode/narrow-screen journeys under lock contention, not uncoached human usability. Latency includes rendering and Python observation.'}
  for field in ['input_to_visible_ms','cancel_to_visible_ms']:
   values=sorted(r[field] for r in rows if field in r)
   report[field]={'median':statistics.median(values),'p95':values[min(len(values)-1,int(len(values)*.95))],'max':max(values)} if values else None
  a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
  if not row['passed']:print(row['error'],flush=True)
print(f"{report['passed']}/{len(rows)} terminal journeys passed")
raise SystemExit(0 if report['passed']==len(rows) else 1)
