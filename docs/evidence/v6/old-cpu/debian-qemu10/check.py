import hashlib,json,os,pathlib,subprocess,time,urllib.request
out=pathlib.Path('/results');out.mkdir(exist_ok=True)
rows=[]
def command(name,args,timeout=45):
 start=time.monotonic()
 try:
  r=subprocess.run(args,capture_output=True,text=True,timeout=timeout)
  row={'name':name,'argv':args,'exit':r.returncode,'passed':r.returncode==0,'seconds':round(time.monotonic()-start,3),'stdout':r.stdout,'stderr':r.stderr}
 except subprocess.TimeoutExpired as e:
  row={'name':name,'argv':args,'exit':None,'passed':False,'seconds':round(time.monotonic()-start,3),'timeout':True,'stdout':str(e.stdout or ''),'stderr':str(e.stderr or '')}
 rows.append(row);print(json.dumps({k:row[k] for k in ['name','passed','seconds']}),flush=True)
 return row
command('emulator-version',['qemu-x86_64','--version'])
for cpu in ['Nehalem','Penryn','qemu64']:
 prefix=['qemu-x86_64','-cpu',cpu]
 command(cpu+'-alt-version',prefix+['/alt','--version'])
 command(cpu+'-alt-practice',prefix+['/alt','--data-dir','/tmp/alt-'+cpu,'practice'])
 command(cpu+'-goose-version',prefix+['/goose','--version'])
 command(cpu+'-llama-version',prefix+['/runtime/llama-server','--version'])
argv=['qemu-x86_64','-cpu','Nehalem','/runtime/llama-server','--model','/model.gguf','--ctx-size','512','--threads','2','--batch-size','32','--ubatch-size','32','--parallel','1','--gpu-layers','0','--port','18819','--host','127.0.0.1']
start=time.monotonic();log=out/'runtime.log'
p=subprocess.Popen(argv,stdout=log.open('w'),stderr=subprocess.STDOUT)
row={'name':'Nehalem-selected-uncensored-generation','argv':argv,'passed':False}
try:
 deadline=time.monotonic()+180
 while time.monotonic()<deadline:
  if p.poll() is not None:raise RuntimeError('Runtime exited: '+str(p.returncode))
  try:
   with urllib.request.urlopen('http://127.0.0.1:18819/health',timeout=2) as r:
    if r.status==200:break
  except Exception:time.sleep(.3)
 else:raise TimeoutError('Runtime health was unavailable within 180 seconds')
 row['load_seconds']=round(time.monotonic()-start,3)
 request=urllib.request.Request('http://127.0.0.1:18819/completion',data=json.dumps({'prompt':'Reply with only OK.','n_predict':8,'temperature':0,'seed':42,'cache_prompt':False}).encode(),headers={'Content-Type':'application/json'})
 with urllib.request.urlopen(request,timeout=180) as response:row['response']=json.load(response)
 row['passed']=row['response'].get('tokens_predicted',0)>0
except Exception as e:row['error']=str(e)
finally:
 p.terminate()
 try:p.wait(timeout=10)
 except subprocess.TimeoutExpired:p.kill();p.wait()
 row['seconds']=round(time.monotonic()-start,3);row['process_reaped']=p.poll() is not None;rows.append(row)
report={'schema':1,'checks':rows,'model_sha256':hashlib.sha256(pathlib.Path('/model.gguf').read_bytes()).hexdigest(),'alt_sha256':hashlib.sha256(pathlib.Path('/alt').read_bytes()).hexdigest(),'goose_sha256':hashlib.sha256(pathlib.Path('/goose').read_bytes()).hexdigest(),'runtime_sha256':hashlib.sha256(pathlib.Path('/runtime/llama-server').read_bytes()).hexdigest(),'passed':all(r['passed'] for r in rows),'scope':'QEMU user-mode CPU instruction compatibility probes for Nehalem, Penryn and qemu64 without AVX, plus one raw-completion forward-generation probe on Nehalem using explicitly selected SHA-pinned uncensored Spark. Container Debian 13, network disabled except loopback. Not physical PC qualification, throughput, full application coverage, GPU support or a model task score.'}
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':report['passed'],'checks':len(rows)}),flush=True)
raise SystemExit(0 if report['passed'] else 1)
