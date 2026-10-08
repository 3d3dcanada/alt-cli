import sys,json,tempfile,time,threading,subprocess
from pathlib import Path
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
sys.path.insert(0,'/workspace/alt-cli/scripts')
from terminal_harness import Terminal
out=Path('/workspace/.alt-audit-ux');binary=Path('/workspace/alt-cli/target/debug/alt')
started=threading.Event()
class Inventory(BaseHTTPRequestHandler):
 def log_message(self,*args):pass
 def do_GET(self):
  started.set();time.sleep(3);body=b'{"data":[{"id":"fixture"}]}'
  self.send_response(200);self.send_header('Content-Length',str(len(body)));self.end_headers()
  try:self.wfile.write(body)
  except BrokenPipeError:pass
server=ThreadingHTTPServer(('127.0.0.1',0),Inventory);threading.Thread(target=server.serve_forever,daemon=True).start()
with tempfile.TemporaryDirectory(prefix='alt-inventory-cancel-') as d:
 root=Path(d);project=root/'project';project.mkdir();state=root/'state'
 subprocess.run([str(binary),'--data-dir',str(state),'init','--model','fixture','--endpoint',f'http://127.0.0.1:{server.server_port}/v1'],check=True,capture_output=True)
 t=Terminal(binary,state,project)
 try:
  t.wait('Start a new conversation');t.send(b'\x1b4');t.wait('Saved connections');t.send(b'e');t.wait('Server address');t.send(b'\r');t.wait('Checking the connection');assert started.wait(2)
  t.send(b'\x1b');t.wait('Saved connections');t.send(b'\x1b');t.wait('Stop requested');
  end=time.monotonic()+.2
  while time.monotonic()<end:t.pump()
  (out/'cancel-inventory-requested.txt').write_text('\n'.join(t.screen.display))
  t.wait('Choose your model');end=time.monotonic()+.2
  while time.monotonic()<end:t.pump()
  (out/'cancel-inventory-result.txt').write_text('\n'.join(t.screen.display))
  (out/'cancel-inventory-result.json').write_text(json.dumps({'cancel_requested':True,'cancelled_request_result_reopened_model_picker':True},indent=2));print('Confirmed: cancelled inventory reopens model picker')
 finally:t.close()
server.shutdown();server.server_close()
