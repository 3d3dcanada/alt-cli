import hashlib,json,os,subprocess,sys,time
from pathlib import Path
root=Path('/workspace/.alt-final-qualification');frozen=root/'baseline';manifest=json.loads((frozen/'manifest.json').read_text())
for item in manifest['attempts']:
 model=next(m for m in manifest['models'] if m['id']==item['model']);attempt=root/'baseline-attempts'/item['id'];attempt.parent.mkdir(exist_ok=True)
 claim=attempt.parent/(item['id']+'.started.json');claim.write_text(json.dumps({'declared_attempt':item,'started_unix':time.time(),'manifest_sha256':hashlib.sha256((frozen/'manifest.json').read_bytes()).hexdigest()}))
 c=manifest['conditions'];cmd=[sys.executable,str(frozen/'live_acceptance.py'),'--suite','v5','--binary',str(frozen/'alt'),'--engine',manifest['engine'],'--runtime',manifest['runtime'],'--model',model['path'],'--sha256',model['sha256'],'--uncensored','--contexts',str(c['native_context']),'--repeats','1','--cases',item['case'],'--partition','development','--timeout',str(c['timeout_seconds']),'--threads',str(c['threads']),'--batch',str(c['batch']),'--tool-profile',c['tool_profile'],'--verification-plan','--output-tokens',str(c['output_tokens']),'--generated-tokens',str(c['generated_tokens']),'--requests',str(c['requests']),'--temperature',str(c['temperature']),'--top-p',str(c['top_p']),'--output',str(attempt)]
 with (attempt.parent/(item['id']+'.log')).open('x') as log:
  p=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,timeout=1000)
 receipt={'declared_attempt':item,'exit_code':p.returncode,'reports':[str(x.relative_to(root))for x in attempt.glob('*/report.json')],'completed_unix':time.time()}
 (attempt.parent/(item['id']+'.finished.json')).write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt),flush=True)
