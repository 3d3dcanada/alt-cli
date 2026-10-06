#!/usr/bin/env python3
"""Task/check/memory/access workflows through a real PTY, with no model inference."""
import fcntl,json,os,pty,select,signal,sqlite3,struct,subprocess,tempfile,termios,time,tomllib
from pathlib import Path
import pyte
repo=Path(__file__).resolve().parents[1]
binary=Path(os.environ.get('ALT_TEST_BINARY',repo/'target/debug/alt')).resolve()
with tempfile.TemporaryDirectory(prefix='alt-task-tui-') as t:
 root=Path(t);project=root/'project';project.mkdir();state=root/'state';state.mkdir()
 (project/'check.py').write_text("print('TASK_FRESH_CHECK_OK')\n")
 (state/'preferences.toml').write_text('project = '+json.dumps(str(project))+'\n')
 master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,120,0,0));original=termios.tcgetattr(slave)
 proc=subprocess.Popen([str(binary),'--data-dir',str(state)],stdin=slave,stdout=slave,stderr=slave,cwd=project,env={**os.environ,'TERM':'xterm-256color'})
 screen=pyte.Screen(120,40);stream=pyte.ByteStream(screen)
 def pump(timeout=.05):
  if select.select([master],[],[],timeout)[0]:stream.feed(os.read(master,65536))
 def wait(text,timeout=10):
  end=time.monotonic()+timeout
  while time.monotonic()<end:
   pump()
   if text in '\n'.join(screen.display):return
  raise AssertionError('Missing '+repr(text)+'\n'+'\n'.join(screen.display))
 def send(data):
  for _ in range(2):pump(.02)
  os.write(master,data)
 def paste(s):send(b'\x1b[200~'+s.encode()+b'\x1b[201~')
 def capture(name):
  if os.environ.get('ALT_TUI_SCREENSHOTS'):
   from terminal_capture import save_screen
   for _ in range(4):pump(.05)
   save_screen(screen,Path(os.environ['ALT_TUI_SCREENSHOTS'])/name)
 try:
  wait('Connect your first model');send(b'\x1b8');wait('What happened');capture('task-empty')
  # Register a check without running it or needing a model connection.
  send(b'c');wait('Choose what should prove');send(b'\r');wait("Your project's check command");paste('python3 check.py');send(b'\r');wait('Check saved')
  # Guided checks either run isolated or explain why nothing ran; never silently use host access.
  send(b'r');wait('Run checks again');send(b'\r')
  end=time.monotonic()+10
  while time.monotonic()<end:
   pump();dbs=list((state/'projects').glob('*/project.db'))
   if dbs:
    with sqlite3.connect(dbs[0]) as db:rows=list(db.execute('SELECT payload FROM checks'))
    if rows:break
  assert rows,'Check request was not recorded'
  first=json.loads(rows[0][0]);assert first['isolation'].startswith('Bubblewrap')
  if first['error']:assert 'No command ran' in first['error']
  # Full access is an explicit, explained selection in Settings.
  send(b'\x1b6');wait('Preferences');send(b'\x1b[B'*6+b'\r');wait('Choose the access you want')
  send(b'\x1b[B\r');wait('Enable Full access?');capture('full-access');send(b'\t\r');wait('Access saved')
  assert tomllib.loads((state/'preferences.toml').read_text())['access_policy']=='trusted'
  send(b'\x1b8');wait('What happened');send(b'r');wait('Run checks again');send(b'\r');wait('passed on current files')
  send(b'e');wait('Check evidence');wait('Passed');wait('Actual check output:');wait('TASK_FRESH_CHECK_OK');capture('task-evidence');send(b'\r')
  # A user decision survives both project memory retrieval and a restart.
  send(b'n');wait('Remember a decision');paste('Keep the calculator public API unchanged.');send(b'\x13');wait('Decision saved')
  send(b'm');wait('Search project memory');send(b'\x15');paste('calculator');send(b'\r');wait('Project memory and current files');send(b'\x1b[6~');wait('public API');wait('unchanged.');capture('task-memory');send(b'\r')
  send(b'\x11');proc.wait(timeout=10);assert proc.returncode==0;assert termios.tcgetattr(slave)==original
  exported=subprocess.check_output([str(binary),'--data-dir',str(state),'task','export'],cwd=project,text=True)
  assert 'public API unchanged' in exported and 'TASK_FRESH_CHECK_OK' in exported
  with sqlite3.connect(dbs[0]) as db:results=[json.loads(v) for (v,) in db.execute('SELECT payload FROM checks')]
  assert len(results)==2 and results[-1]['exit_code']==0
  print('PASS: Task page, configure/rerun checks without a model, isolation failure reporting, explicit Full access, raw evidence, durable decisions, memory retrieval, export, terminal restoration.')
 finally:
  if proc.poll() is None:proc.terminate();proc.wait(timeout=10)
  os.close(master);os.close(slave)
