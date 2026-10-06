import hashlib,json,os,subprocess,threading,time,urllib.request
from pathlib import Path
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
repo=Path('/workspace/alt-cli');root=Path('/workspace/.alt-evaluations/v4');endpoint='http://127.0.0.1:11438';requests=[];records=[]
while not (root/'candidate-followups.json').exists():time.sleep(3)
binary=repo/'target/portable-glibc231/release/alt';sha='5d38e532cf33a5b56760bc7803aaabffb240d0604f1e389e4db148830c2bb747';model='/workspace/.alt-models/Qwen3-4B-Instruct-2507-heretic.Q4_K_M.gguf'
def api(path,data):
 request=urllib.request.Request(endpoint+path,data=json.dumps(data).encode(),headers={'Content-Type':'application/json'})
 with urllib.request.urlopen(request,timeout=120)as response:return json.load(response)
class Relay(BaseHTTPRequestHandler):
 def log_message(self,*_):pass
 def do_GET(self):self.forward()
 def do_POST(self):self.forward()
 def forward(self):
  data=self.rfile.read(int(self.headers.get('Content-Length','0'))) if self.command=='POST' else None
  if data:
   body=json.loads(data);requests.append({'path':self.path,'model':body.get('model'),'max_tokens':body.get('max_tokens'),'has_native_options': 'options' in body})
  request=urllib.request.Request(endpoint+self.path,data=data,headers={'Content-Type':'application/json'},method=self.command)
  try:
   with urllib.request.urlopen(request,timeout=270)as response:
    self.send_response(response.status)
    for k,v in response.headers.items():
     if k.lower()not in ['connection','transfer-encoding','content-length']:self.send_header(k,v)
    self.end_headers()
    while True:
     chunk=response.read1(8192)
     if not chunk:break
     self.wfile.write(chunk);self.wfile.flush()
  except(BrokenPipeError,ConnectionResetError):pass
server=ThreadingHTTPServer(('127.0.0.1',0),Relay);server.daemon_threads=True;threading.Thread(target=server.serve_forever,daemon=True).start();relay='http://127.0.0.1:'+str(server.server_port)
try:
 for context in [4096,16384]:
  tag='alt-heretic-qwen3-4b-context-'+str(context)
  print('Validating corrected Ollama chat with actual native window',context,flush=True)
  api('/api/create',{'model':tag,'from':'alt-heretic-qwen3-4b','parameters':{'num_ctx':context,'temperature':0},'stream':False})
  shown=api('/api/show',{'model':tag});assert sha in shown['modelfile'],'Wrong backing artifact'
  output=root/('corrected-ollama-'+str(context))
  command=['python3','scripts/live_acceptance.py','--binary',str(binary),'--engine','/workspace/.alt-tools/goose','--provider','ollama','--endpoint',relay,'--model-id',tag,'--model',model,'--sha256',sha,'--uncensored','--verification-plan','--history-notes','40','--contexts',str(context),'--repeats','1','--cases','python-dependency','--timeout','240','--output',str(output)]
  result=subprocess.run(command,cwd=repo)
  report=json.loads(next(output.glob('*/report.json')).read_text());matching=[r for r in requests if r['path']=='/v1/chat/completions' and r['model']==tag]
  verified=any(r.get('context_length')==context for r in report.get('server_runtime_state',[]))
  records.append({'context':context,'tag':tag,'native_context_verified_by_api_ps':verified,'native_state':report.get('server_runtime_state'),'chat_budget_correct':bool(matching)and all(r['max_tokens']==min(context//4,2048) and not r['has_native_options']for r in matching),'passed_source':report['passed'],'cli_exit':report['cli_exit'],'suite_exit':result.returncode})
  api('/api/generate',{'model':tag,'keep_alive':0,'stream':False})
finally:server.shutdown();server.server_close()
(root/'corrected-ollama-compatibility.json').write_text(json.dumps({'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'real_server':endpoint,'proxy':'Loopback transparent streaming relay records request limits only; actual server executes inference','records':records,'requests':requests},indent=2)+'\n');print('Corrected Ollama compatibility checks complete:',records,flush=True)
