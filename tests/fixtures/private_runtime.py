#!/usr/bin/env python3
"""Owned-runtime protocol fixture. It has no weights and performs no inference."""
import http.server
import json
import os
import socketserver
import sys
import time

if '--list-devices' in sys.argv:
    print('Available devices:')
    raise SystemExit(0)
if '--help' in sys.argv:
    print('--host UNIX socket paths ending in .sock --reasoning-budget')
    raise SystemExit(0)
socket_path = sys.argv[sys.argv.index('--host') + 1]
if os.environ.get('ALT_FIXTURE_NEVER_READY') == '1':
    time.sleep(60)
class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'
    def do_GET(self):
        data = json.dumps({'status': 'ok', 'fixture_pid': os.getpid()}).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)
    def do_POST(self):
        self.rfile.read(int(self.headers.get('Content-Length', 0)))
        self.do_GET()
    def log_message(self, *args):
        pass
class Server(socketserver.UnixStreamServer):
    pass
print('fixture: offloaded 0/2 layers to GPU', flush=True)
with Server(socket_path, Handler) as server:
    server.serve_forever()
