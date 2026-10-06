#!/usr/bin/env python3
"""Actual SSH and tmux PTY acceptance, using a disposable loopback-only container."""
import argparse,fcntl,json,os,pty,select,struct,subprocess,tempfile,termios,time,uuid
from pathlib import Path
import pyte
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--alt',type=Path,required=True);p.add_argument('--output',type=Path);a=p.parse_args();binary=a.alt.resolve();records=[];name='alt-terminal-'+uuid.uuid4().hex[:10]
def docker(*args):return subprocess.check_output(['docker',*args],text=True).strip()
with tempfile.TemporaryDirectory(prefix='alt-ssh-') as t:
 root=Path(t);key=root/'identity';subprocess.run(['ssh-keygen','-q','-t','ed25519','-N','','-f',str(key)],check=True)
 try:
  docker('run','-d','--name',name,'-p','127.0.0.1::2222','-v',str(binary)+':/alt:ro','-v',str(key)+'.pub:/tmp/authorized_keys:ro','alt-terminal-host','sh','-c',"mkdir -p /run/sshd /tmp/project; passwd -d root; ssh-keygen -q -t ed25519 -N '' -f /tmp/hostkey; exec /usr/sbin/sshd -D -e -p 2222 -h /tmp/hostkey -o AuthorizedKeysFile=/tmp/authorized_keys -o StrictModes=no -o PasswordAuthentication=no -o PermitEmptyPasswords=no -o PermitRootLogin=prohibit-password -o UsePAM=no")
  port=docker('port',name,'2222/tcp').rsplit(':',1)[1]
  for _ in range(50):
   result=subprocess.run(['docker','exec',name,'cat','/tmp/hostkey.pub'],capture_output=True,text=True)
   if result.returncode==0:break
   time.sleep(.1)
  assert result.returncode==0,result.stderr
  known=root/'known_hosts';known.write_text('[127.0.0.1]:'+port+' '+result.stdout)
  ssh=['ssh','-F','/dev/null','-tt','-i',str(key),'-p',port,'-o','IdentitiesOnly=yes','-o','StrictHostKeyChecking=yes','-o','UserKnownHostsFile='+str(known),'root@127.0.0.1']
  for mode in ['ssh','ssh-tmux']:
   command='cd /tmp/project && exec '+('/alt --data-dir /tmp/state-ssh' if mode=='ssh' else "tmux -L alt-smoke new-session -s acceptance '/alt --data-dir /tmp/state-tmux'")
   master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',32,100,0,0));original=termios.tcgetattr(slave);screen=pyte.Screen(100,32);stream=pyte.ByteStream(screen);proc=subprocess.Popen(ssh+[command],stdin=slave,stdout=slave,stderr=slave,env={**os.environ,'TERM':'xterm-256color'})
   def pump():
    if select.select([master],[],[],.05)[0]:
     try:stream.feed(os.read(master,65536))
     except OSError:pass
   def wait(text):
    end=time.monotonic()+12
    while time.monotonic()<end:
     pump()
     if text in '\n'.join(screen.display):return
    raise AssertionError(mode+': missing '+text+'\n'+'\n'.join(screen.display))
   try:
    wait('Connect your first model');os.write(master,b'\x1b9');wait('Project files');os.write(master,b'n');wait('Create a file');os.write(master,b'\x1b[200~'+mode.encode()+b'.txt\x1b[201~\r');wait('Edit '+mode+'.txt');os.write(master,b'\x1b[200~Hello over '+mode.encode()+b'\x1b[201~\x13');wait('Save '+mode+'.txt?');os.write(master,b'\t\r');wait('File saved with an undo checkpoint')
    assert docker('exec',name,'cat','/tmp/project/'+mode+'.txt')=='Hello over '+mode
    os.write(master,b'\x11');proc.wait(timeout=10);assert proc.returncode==0;assert termios.tcgetattr(slave)==original;records.append({'mode':mode,'fresh_home':True,'navigation_and_editor':True,'checkpoint_save':True,'actual_file_verified':True,'clean_exit_and_terminal_restore':True})
   finally:
    if proc.poll() is None:proc.terminate();proc.wait(timeout=5)
    os.close(master);os.close(slave)
  print('PASS: actual SSH and SSH+tmux, first-run navigation, file editor/checkpoint, clean exit and terminal settings')
 finally:
  subprocess.run(['docker','rm','-f',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
  if a.output:a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps({'scope':'Loopback SSH with pinned generated host key; disposable Debian 12 container; installed portable binary','records':records},indent=2)+'\n')
