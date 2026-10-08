import sys, json, tempfile, time, threading, subprocess, tomllib
from pathlib import Path
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
sys.path.insert(0,'/workspace/alt-cli/scripts')
from terminal_harness import Terminal
out=Path('/workspace/.alt-audit-ux')
binary=Path('/workspace/alt-cli/target/debug/alt')
requests=[]
class Inventory(BaseHTTPRequestHandler):
 def log_message(self,*args): pass
 def do_GET(self):
  requests.append(self.path)
  body=json.dumps({'data':[{'id':'fixture'}]}).encode()
  self.send_response(200);self.send_header('Content-Length',str(len(body)));self.end_headers();self.wfile.write(body)
server=ThreadingHTTPServer(('127.0.0.1',0),Inventory)
threading.Thread(target=server.serve_forever,daemon=True).start()
results={}
def drain(t):
 end=time.monotonic()+.35
 while time.monotonic()<end:t.pump(.02)
def capture(t,name):
 drain(t);text='\n'.join(t.screen.display);(out/(name+'.txt')).write_text(text);return text
with tempfile.TemporaryDirectory(prefix='alt-ux-audit-') as tmp:
 root=Path(tmp);project=root/'project';project.mkdir();state=root/'state'
 t=Terminal(binary,state,project,80,24)
 try:
  t.wait('Connect your first model');t.send(b'\x1b2');t.wait('Describe what you want');t.paste('DRAFT-UNSENT-123');t.wait('DRAFT-UNSENT-123');capture(t,'draft-before-new')
  t.send(b'\x0e');t.wait('New conversation');results['ctrl_n_clears_draft']='DRAFT-UNSENT-123' not in capture(t,'draft-after-new')
  t.paste('DRAFT-RESTART-456');t.wait('DRAFT-RESTART-456');t.close()
  t=Terminal(binary,state,project,80,24);t.wait('Connect your first model');t.send(b'\x1b2');t.wait('Describe what you want');results['quit_restart_loses_draft']='DRAFT-RESTART-456' not in capture(t,'draft-after-restart')
 finally:t.close()
 # Inventory uses local deterministic fixture, no model.
 state=root/'connection-state';base=[str(binary),'--data-dir',str(state)]
 subprocess.run(base+['init','--model','fixture','--endpoint',f'http://127.0.0.1:{server.server_port}/v1','--context','4096','--uncensored'],check=True,capture_output=True)
 subprocess.run(base+['inference','--output-tokens','256','--action-headroom','128','--temperature','0.35','--requests','7','--generated-tokens','2048'],check=True,capture_output=True)
 before=tomllib.loads((state/'config.toml').read_text());(out/'connection-before.json').write_text(json.dumps(before,indent=2))
 t=Terminal(binary,state,project,80,24)
 try:
  t.wait('Start a new conversation');t.send(b'\x1b4');t.wait('Saved connections');t.send(b'e');t.wait('Server address');capture(t,'edit-connection-before')
  count=len(requests);t.send(b'\x0d');t.wait('Choose your model');results['ctrl_m_issues_inventory_request']=len(requests)==count+1;results['ctrl_m_shows_manual_id']='Enter a model identifier' in capture(t,'ctrl-m-result')
  t.send(b'\r');t.wait('Connection saved');after=tomllib.loads((state/'config.toml').read_text());(out/'connection-after.json').write_text(json.dumps(after,indent=2));results['editing_profile']={'before':before,'after':after}
 finally:t.close()
 for width,height in [(60,18),(80,24)]:
  t=Terminal(binary,root/f'small-{width}',project,width,height)
  try:
   t.wait('Connect your first model');t.send(b'\r');t.wait('Where will your model run?');t.send(b'\x1b[B\r');t.wait('Server address');text=capture(t,f'connection-{width}x{height}');results[f'connection_{width}x{height}']={'api_key_label_visible':'API key variable' in text,'manual_id_hint_visible':'Ctrl+M' in text,'check_button_visible':'Check connection' in text}
  finally:t.close()
server.shutdown();server.server_close()
(out/'results.json').write_text(json.dumps(results,indent=2));print(json.dumps(results,indent=2))
