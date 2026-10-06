#!/usr/bin/env python3
"""Build a minimal current-Linux installer host using existing proxy settings."""
import os, socket, subprocess, urllib.parse
from pathlib import Path
command=['docker','build'];hosts=set()
for name in ['HTTP_PROXY','HTTPS_PROXY','NO_PROXY']:
    if os.environ.get(name):
        command+=['--build-arg',name]
        if name!='NO_PROXY':
            host=urllib.parse.urlsplit(os.environ[name]).hostname
            if host and host not in hosts:
                command+=['--add-host',host+':'+socket.gethostbyname(host)];hosts.add(host)
subprocess.run(command+['-t','alt-release-current',str(Path(__file__).resolve().parent/'release-host')],check=True)
