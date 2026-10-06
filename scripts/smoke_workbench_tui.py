#!/usr/bin/env python3
"""Real terminal acceptance for files, checkpoints, jobs, context and management."""
import fcntl,json,os,pty,select,sqlite3,struct,subprocess,tempfile,termios,time,threading
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path
import pyte
repo=Path(__file__).resolve().parents[1];binary=Path(os.environ.get('ALT_TEST_BINARY',repo/'target/debug/alt')).resolve()
class Terminal:
 def __init__(self,state,project):
  self.master,self.slave=pty.openpty();fcntl.ioctl(self.slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,120,0,0));self.original=termios.tcgetattr(self.slave);self.screen=pyte.Screen(120,40);self.stream=pyte.ByteStream(self.screen)
  self.proc=subprocess.Popen([str(binary),'--data-dir',str(state)],cwd=project,stdin=self.slave,stdout=self.slave,stderr=self.slave,env={**os.environ,'TERM':'xterm-256color'})
 def pump(self,timeout=.03):
  if select.select([self.master],[],[],timeout)[0]:
   try:self.stream.feed(os.read(self.master,65536))
   except OSError:pass
 def wait(self,text,timeout=12):
  end=time.monotonic()+timeout
  while time.monotonic()<end:
   self.pump()
   if text in '\n'.join(self.screen.display):return
   if self.proc.poll() is not None:break
  raise AssertionError(f'Missing {text!r}; exit {self.proc.poll()}\n'+'\n'.join(self.screen.display))
 def send(self,data):
  for _ in range(3):self.pump(.03)
  os.write(self.master,data)
 def paste(self,text):self.send(b'\x1b[200~'+text.encode()+b'\x1b[201~')
 def settings(self,index):self.send(b'\x1b6');self.wait('Preferences');self.send(b'\x1b[A'*20+b'\x1b[B'*index+b'\r')
 def quit(self):
  self.send(b'\x11');self.proc.wait(timeout=10);assert self.proc.returncode==0;assert termios.tcgetattr(self.slave)==self.original;os.close(self.master);os.close(self.slave)
 def cleanup(self):
  if self.proc.poll() is None:self.proc.terminate();self.proc.wait(timeout=10)
 def capture(self,name):
  if os.environ.get('ALT_TUI_SCREENSHOTS'):
   from terminal_capture import save_screen
   for _ in range(5):self.pump(.03)
   save_screen(self.screen,Path(os.environ['ALT_TUI_SCREENSHOTS'])/name)
class Health(BaseHTTPRequestHandler):
 def log_message(self,*args):pass
 def do_GET(self):self.send_response(200);self.send_header('Content-Length','2');self.end_headers();self.wfile.write(b'OK')
health=ThreadingHTTPServer(('127.0.0.1',0),Health);threading.Thread(target=health.serve_forever,daemon=True).start()
with tempfile.TemporaryDirectory(prefix='alt-workbench-tui-') as t:
 root=Path(t);project=root/'project';project.mkdir();state=root/'state';state.mkdir();(state/'preferences.toml').write_text(f'project = {json.dumps(str(project))}\naccess_policy = "trusted"\n')
 term=Terminal(state,project);job=None
 try:
  term.wait('Connect your first model');term.send(b'\x1b9');term.wait('Project files');term.send(b'n');term.wait('Create a file');term.paste('notes.txt');term.send(b'\r');term.wait('Edit notes.txt');term.paste('Cedar project notes\nPreserve the public API.\n');term.send(b'\x13');term.wait('Save notes.txt?');assert not (project/'notes.txt').exists();term.capture('file-preview');term.send(b'\t\r');term.wait('File saved with an undo checkpoint');assert (project/'notes.txt').read_text().startswith('Cedar')
  term.send(b'\r');term.wait('Cedar project notes');term.send(b'\r');term.send(b's');term.wait('Find text in notes.txt');term.paste('public API');term.send(b'\r');term.wait('1 matching lines');term.send(b'\r');term.send(b'g');term.wait('Go to line in notes.txt');term.paste('2');term.send(b'\r');term.wait('from 2');term.send(b'\r');term.send(b'/');term.wait('Filter project files');term.paste('missing');term.send(b'\r');term.send(b'\r');term.wait("Let's get this working");term.send(b'\r')
  # Existing empty file is editable, not an invalid text-replacement trap.
  (project/'empty.txt').touch();term.send(b'/');term.wait('Filter project files');term.send(b'\x15');term.paste('empty');term.send(b'\r');term.send(b'r');time.sleep(.2);term.send(b'e');term.wait('Edit empty.txt');term.paste('No longer empty');term.send(b'\x13');term.wait('Save empty.txt?');term.send(b'\t\r');term.wait('File saved with an undo checkpoint');assert (project/'empty.txt').read_text()=='No longer empty'
  term.send(b'\x10');term.wait('Quick actions');term.send(b'\x1b[B'*10+b'\r');term.wait('Context');term.send(b'p');term.wait('Pin an important requirement');term.paste('Keep Cedar protocol backward compatible.');term.send(b'\x13');term.wait('Requirement pinned');term.capture('context')
  # Settings expose real configuration wizards, not TOML editing.
  term.settings(10);term.wait('Tools and workflows');term.send(b'\x1b[B\r');term.wait('External MCP connections');term.send(b'\r');term.wait('Name this tool connection');term.paste('fixture');term.send(b'\r');term.wait('How does this server connect?');term.send(b'\r');term.wait('Server launch command');term.paste('python3 '+str(repo/'tests/fixtures/mcp_server.py'));term.send(b'\r');term.wait('Check connection and choose tools');term.send(b'\r');term.wait('Choose tools to expose');term.send(b'\r');term.wait('[selected] counter');term.capture('extensions');term.send(b'\x1b')
  term.settings(11);term.wait('Storage and recovery');term.send(b'\x1b[B'*3+b'\r');term.wait('Save this redacted diagnostics report?');term.send(b'\t\r');term.wait('Saved');assert list((state/'exports').glob('diagnostics-*.json'))
  term.send(b'\x1b0');term.wait('Terminal jobs');term.send(b'n');term.wait('Start an interactive command');term.paste("printf 'JOB_READY\\n'; read answer; printf 'REPLY_%s\\n' \"$answer\"; sleep 60");term.send(b'\r');term.wait('How long should this job live?');term.send(b'\x1b[B\r');term.wait('Job time budget');term.send(b'\r');term.wait('JOB_READY');term.send(b'\r');term.wait('Terminal input is active');term.paste('hello');term.send(b'\r');term.wait('REPLY_hello');term.capture('terminal');term.send(b'\x1d');term.wait('Detached from input');term.send(b'h');term.wait("Check this service's health");term.send(b'\x15');term.paste(f'http://127.0.0.1:{health.server_port}/');term.send(b'\r');term.wait('Service is responding');term.send(b'\r')
  records=[json.loads(p.read_text()) for p in (state/'jobs').glob('*/job.json')];job=records[0]['id'];assert records[0]['spec']['keep'];term.quit()
  # Job is owned by a supervisor, survives Alt restart, and can then be stopped.
  term=Terminal(state,project);term.wait('Connect your first model');term.send(b'\x1b0');term.wait('running');term.send(b'\r');term.wait('REPLY_hello');term.send(b'\x1d');term.send(b's');term.wait('stopped');record=json.loads((state/'jobs'/job/'job.json').read_text());assert record['status']=='stopped'
  # Undo the whole manual task independently of a model.
  term.send(b'\x1b8');term.wait('What happened');term.send(b'U');term.wait("Undo this task's file edits?");term.send(b'\t\r');term.wait('Tracked file edits restored');assert not (project/'notes.txt').exists();assert (project/'empty.txt').read_text()==''
  fcntl.ioctl(term.slave,termios.TIOCSWINSZ,struct.pack('HHHH',18,60,0,0));term.send(b'\x1b6');term.wait('Preferences');term.quit()
  print('PASS: file creation/edit/preview/undo, empty filtered list, empty-file edit, pinned context, MCP setup/tool selection, diagnostics, PTY input/persistence/reconnect/stop, narrow terminal and restored terminal settings')
 finally:
  term.cleanup();health.shutdown()
  if job:
   subprocess.run([str(binary),'--data-dir',str(state),'jobs','stop',job],capture_output=True)
