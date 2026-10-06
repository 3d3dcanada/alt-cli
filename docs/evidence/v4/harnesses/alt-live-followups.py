import hashlib,json,os,subprocess,time,urllib.request
from pathlib import Path
repo=Path('/workspace/alt-cli');root=Path('/workspace/.alt-evaluations/v4');pid=78472
stat=Path(f'/proc/{pid}/stat')
identity=stat.read_text().rsplit(') ',1)[1].split()[19] if stat.exists() else None
print('Waiting for the baseline matrix to release the inference runtimes.',flush=True)
while identity:
 try:
  fields=stat.read_text().rsplit(') ',1)[1].split()
  if fields[19]!=identity or fields[0]=='Z':break
 except OSError:break
 time.sleep(3)
binary=repo/'target/portable-glibc231/release/alt';engine=Path('/workspace/.alt-tools/goose');runtime=Path('/workspace/.alt-tools/llama-b11429/llama-b11429/llama-server');model=Path('/workspace/.alt-models/Qwen3-4B-Instruct-2507-heretic.Q4_K_M.gguf');sha='5d38e532cf33a5b56760bc7803aaabffb240d0604f1e389e4db148830c2bb747';outcomes=[]
for provider in ['llama','ollama']:
 destination=root/('final-'+provider+'-verification-context')
 args=['python3','scripts/live_acceptance.py','--binary',str(binary),'--engine',str(engine),'--model',str(model),'--sha256',sha,'--uncensored','--verification-plan','--history-notes','40','--contexts','4096,16384','--repeats','1','--cases','python-feature,python-dependency','--timeout','240','--output',str(destination)]
 args+=['--runtime',str(runtime)] if provider=='llama' else ['--provider','ollama','--endpoint','http://127.0.0.1:11438','--model-id','alt-heretic-qwen3-4b']
 print('Starting final-build required-verification/context runs:',provider,flush=True)
 result=subprocess.run(args,cwd=repo);outcomes.append({'suite':provider,'exit':result.returncode})
# Unload the external model while measuring the managed runtime alone.
try:
 req=urllib.request.Request('http://127.0.0.1:11438/api/generate',data=json.dumps({'model':'alt-heretic-qwen3-4b','keep_alive':0,'stream':False}).encode(),headers={'Content-Type':'application/json'})
 urllib.request.urlopen(req,timeout=30).read()
except Exception as e:print('Ollama unload:',type(e).__name__,flush=True)
for provider in ['llama','ollama']:
 state=root/('benchmark-'+provider+'-state');state.mkdir(exist_ok=False)
 if provider=='llama':(state/'preferences.toml').write_text('context_tokens=8192\nruntime_path='+json.dumps(str(runtime))+'\n')
 base=[str(binary),'--data-dir',str(state),'--engine',str(engine)]
 args=['models','import',str(model),'--uncensored','--use']if provider=='llama'else['init','--provider','ollama','--endpoint','http://127.0.0.1:11438','--model','alt-heretic-qwen3-4b','--context','8192','--uncensored']
 setup=subprocess.run(base+args,cwd=repo,capture_output=True,text=True,timeout=180);assert setup.returncode==0,setup.stderr
 print('Measuring load/generation:',provider,flush=True)
 with (root/('benchmark-'+provider+'.json')).open('w')as output,(root/('benchmark-'+provider+'.stderr')).open('w')as error:
  result=subprocess.run(base+['benchmark'],cwd=repo,stdout=output,stderr=error,timeout=660)
 outcomes.append({'benchmark':provider,'exit':result.returncode})
 if provider=='ollama':
  with (root/'ollama-capability.json').open('w')as output,(root/'ollama-capability.stderr').open('w')as error:
   result=subprocess.run(base+['evaluate'],cwd=repo,stdout=output,stderr=error,timeout=240)
  outcomes.append({'native_capability':provider,'exit':result.returncode})
(root/'final-followups.json').write_text(json.dumps({'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'outcomes':outcomes},indent=2)+'\n');print('Final live followups complete:',outcomes,flush=True)
