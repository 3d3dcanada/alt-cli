#!/usr/bin/env python3
"""Build the optional SSH/tmux/isolation test host through the existing proxy/trust."""
import os,shutil,socket,subprocess,tempfile,urllib.parse
from pathlib import Path
repo=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='alt-terminal-build-')as t:
 root=Path(t);recipe=(repo/'scripts/terminal-host/Dockerfile').read_text();trusted=Path('/usr/local/share/ca-certificates')
 if trusted.exists():
  shutil.copytree(trusted,root/'trusted-certificates');recipe=recipe.replace('RUN sed','COPY trusted-certificates/ /usr/local/share/ca-certificates/\nRUN update-ca-certificates && sed',1)
 (root/'Dockerfile').write_text(recipe)
 command=['docker','build'];hosts=set()
 for name in ['HTTP_PROXY','HTTPS_PROXY','NO_PROXY']:
  if os.environ.get(name):
   command+=['--build-arg',name]
   if name!='NO_PROXY':
    host=urllib.parse.urlsplit(os.environ[name]).hostname
    if host and host not in hosts:command+=['--add-host',host+':'+socket.gethostbyname(host)];hosts.add(host)
 subprocess.run(command+['-t','alt-terminal-host',str(root)],check=True)
