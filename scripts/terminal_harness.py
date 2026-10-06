"""A real PTY with bounded waits; shared by targeted UI acceptance journeys."""
import fcntl,os,pty,select,struct,subprocess,termios,time
import pyte
class Terminal:
 def __init__(self,binary,state,project,width=120,height=40):
  self.master,self.slave=pty.openpty();fcntl.ioctl(self.slave,termios.TIOCSWINSZ,struct.pack('HHHH',height,width,0,0));self.original=termios.tcgetattr(self.slave);self.screen=pyte.Screen(width,height);self.stream=pyte.ByteStream(self.screen)
  self.proc=subprocess.Popen([str(binary),'--data-dir',str(state)],cwd=project,stdin=self.slave,stdout=self.slave,stderr=self.slave,env={**os.environ,'TERM':'xterm-256color'})
 def pump(self,seconds=.03):
  if select.select([self.master],[],[],seconds)[0]:self.stream.feed(os.read(self.master,65536))
 def wait(self,text,timeout=10):
  end=time.monotonic()+timeout
  while time.monotonic()<end:
   self.pump()
   if text in '\n'.join(self.screen.display):return
  raise AssertionError('Missing '+repr(text)+'\n'+'\n'.join(self.screen.display))
 def send(self,data):
  for _ in range(3):self.pump(.03)
  os.write(self.master,data)
 def paste(self,text):self.send(b'\x1b[200~'+text.encode()+b'\x1b[201~')
 def close(self):
  if self.proc.poll() is None:self.send(b'\x11');self.proc.wait(timeout=10)
  assert self.proc.returncode==0 and termios.tcgetattr(self.slave)==self.original
  os.close(self.master);os.close(self.slave)
