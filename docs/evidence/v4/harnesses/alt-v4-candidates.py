import concurrent.futures,hashlib,json,os,subprocess,time,urllib.request
from pathlib import Path
repo=Path('/workspace/alt-cli');binary=repo/'target/portable-glibc231/release/alt';root=Path('/workspace/.alt-evaluations/v4');cache=Path('/tmp/alt-v4-candidate-models');cache.mkdir(exist_ok=True);engine='/workspace/.alt-tools/goose';runtime='/workspace/.alt-tools/llama-b11429/llama-b11429/llama-server'
meta=json.loads((repo/'docs/research/2026-10-06-variant-candidates.json').read_text())['candidates'];selected=[('spark',meta[0]),('mimo',meta[2])]
records=[]
def download(pair):
 name,c=pair;f=next(f for f in c['files']if 'Q4_K_M' in f['rfilename']);state=cache/name;out=root/('candidate-'+name);out.mkdir(exist_ok=True)
 if (out/'artifact.json').exists():
  previous=json.loads((out/'artifact.json').read_text())
  if previous['download_exit']==0:return previous
 base=[str(binary),'--data-dir',str(state)]
 attempts=[]
 for attempt in range(1,9):
  with (out/f'download-resume-{attempt}.stdout').open('w')as stdout,(out/f'download-resume-{attempt}.stderr').open('w')as stderr:
   p=subprocess.run(base+['models','download',c['repository'],'--file',f['rfilename']],cwd=repo,stdout=stdout,stderr=stderr,timeout=1200)
  attempts.append({'attempt':attempt,'exit':p.returncode})
  print(name,'resume attempt',attempt,'exit',p.returncode,flush=True)
  if p.returncode==0:break
 result={'name':name,'repository':c['repository'],'revision':c['revision'],'file':f['rfilename'],'sha256':f['lfs']['sha256'],'bytes':f['size'],'download_exit':p.returncode,'resume_attempts':attempts,'uncensored':'Publisher-labelled abliterated/Heretic; no fallback','binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()}
 if p.returncode==0:
  files=list(state.rglob('*.gguf'));assert len(files)==1,files;model=files[0];h=hashlib.sha256()
  with model.open('rb')as src:
   for block in iter(lambda:src.read(8*1024*1024),b''):h.update(block)
  assert h.hexdigest()==result['sha256'] and model.stat().st_size==result['bytes'],'Artifact metadata drift'
  result['model_path']=str(model)
 (out/'artifact.json').write_text(json.dumps(result,indent=2)+'\n');print(name,'download result',p.returncode,flush=True);return result
with concurrent.futures.ThreadPoolExecutor(max_workers=2)as pool:records=list(pool.map(download,selected))
print('Waiting for the existing final-build matrix/benchmark before new inference.',flush=True)
while not (root/'final-followups.json').exists():time.sleep(3)
# Release only our own Ollama evaluation model before testing managed runtimes.
request=urllib.request.Request('http://127.0.0.1:11438/api/generate',data=json.dumps({'model':'alt-heretic-qwen3-4b','keep_alive':0,'stream':False}).encode(),headers={'Content-Type':'application/json'})
with urllib.request.urlopen(request,timeout=30)as response:response.read()
for contexts,cases,label in [('4096','python-dependency','corrected-memory-4k'),('16384','python-feature,python-dependency','corrected-memory-16k')]:
 print('Repeating previously preflight-blocked tests:',label,flush=True)
 command=['python3','scripts/live_acceptance.py','--binary',str(binary),'--engine',engine,'--runtime',runtime,'--model','/workspace/.alt-models/Qwen3-4B-Instruct-2507-heretic.Q4_K_M.gguf','--sha256','5d38e532cf33a5b56760bc7803aaabffb240d0604f1e389e4db148830c2bb747','--uncensored','--verification-plan','--history-notes','40','--contexts',contexts,'--repeats','1','--cases',cases,'--timeout','240','--output',str(root/label)]
 subprocess.run(command,cwd=repo)
for record in records:
 name=record['name'];out=root/('candidate-'+name)
 record['live_binary_sha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
 if record['download_exit']!=0:continue
 state=out/'state';state.mkdir();(state/'preferences.toml').write_text('context_tokens=8192\nruntime_path='+json.dumps(runtime)+'\n');base=[str(binary),'--data-dir',str(state),'--engine',engine]
 p=subprocess.run(base+['models','import',record['model_path'],'--uncensored','--use'],capture_output=True,text=True,timeout=180);assert p.returncode==0,p.stderr
 print('Testing exact new artifact:',name,flush=True)
 with (out/'capability.json').open('w')as stdout,(out/'capability.stderr').open('w')as stderr:
  try:r=subprocess.run(base+['evaluate'],cwd=repo,stdout=stdout,stderr=stderr,timeout=240);record['capability_exit']=r.returncode
  except subprocess.TimeoutExpired:record['capability_timeout']=True
 command=['python3','scripts/live_acceptance.py','--binary',str(binary),'--engine',engine,'--runtime',runtime,'--model',record['model_path'],'--sha256',record['sha256'],'--uncensored','--verification-plan','--history-notes','40','--contexts','8192','--repeats','1','--cases','python-multifile,python-dependency','--timeout','240','--output',str(out/'projects')]
 r=subprocess.run(command,cwd=repo);record['project_suite_exit']=r.returncode
 (out/'artifact.json').write_text(json.dumps(record,indent=2)+'\n')
(root/'candidate-followups.json').write_text(json.dumps({'scope':'Exploratory exact-artifact tool and two-project checks, not a repeated reliability benchmark','records':records},indent=2)+'\n');print('Candidate checks complete.',flush=True)
