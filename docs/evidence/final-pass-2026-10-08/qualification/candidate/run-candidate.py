"""One immutable four-cell historical pilot using the pre-change evaluator."""
import argparse,hashlib,json,os,shutil,subprocess,sys,time
from pathlib import Path
sys.dont_write_bytecode=True
root=Path('/workspace/.alt-final-qualification');baseline=root/'baseline'
sys.path.insert(0,str(root/'campaign-v2/evaluator'))
from qualification_campaign import sha,write_new,run_evaluator,file_manifest
parser=argparse.ArgumentParser();parser.add_argument('--candidate',type=Path,required=True);parser.add_argument('--source-identity',required=True);args=parser.parse_args()
manifest=json.loads((baseline/'manifest.json').read_text())
for name,digest in manifest['evaluator_sha256'].items():assert sha(baseline/name)==digest,(name,'frozen evaluator changed')
assert sha(baseline/'alt')==manifest['binary_sha256']
for key in ['engine','runtime']:assert sha(manifest[key])==manifest[key+'_sha256']
candidate=root/'pilot-candidate';candidate.mkdir(exist_ok=False);shutil.copy2(args.candidate,candidate/'alt')
record={'schema':1,'candidate_sha256':sha(candidate/'alt'),'baseline_manifest_sha256':sha(baseline/'manifest.json'),'source_identity':args.source_identity,'conditions':manifest['conditions'],'attempts':manifest['attempts'],'sealed_unix':time.time(),'scope':'Historical development/validation pilot; no final holdout inspection. Shared-cloud compilation confounds latency; no population quality or speed claim.'}
write_new(candidate/'manifest.json',record)
for item in manifest['attempts']:
 model=next(m for m in manifest['models'] if m['id']==item['model']);assert sha(model['path'])==model['sha256']
 attempt=root/'candidate-attempts'/item['id'];attempt.parent.mkdir(exist_ok=True)
 write_new(attempt.parent/(item['id']+'.started.json'),{'declared_attempt':item,'started_unix':time.time(),'manifest_sha256':sha(candidate/'manifest.json')})
 c=manifest['conditions'];command=[sys.executable,str(baseline/'live_acceptance.py'),'--suite','v5','--binary',str(candidate/'alt'),'--engine',manifest['engine'],'--runtime',manifest['runtime'],'--model',model['path'],'--sha256',model['sha256'],'--uncensored','--contexts',str(c['native_context']),'--repeats','1','--cases',item['case'],'--partition','development','--timeout',str(c['timeout_seconds']),'--threads',str(c['threads']),'--batch',str(c['batch']),'--tool-profile',c['tool_profile'],'--verification-plan','--output-tokens',str(c['output_tokens']),'--generated-tokens',str(c['generated_tokens']),'--requests',str(c['requests']),'--temperature',str(c['temperature']),'--top-p',str(c['top_p']),'--output',str(attempt)]
 write_new(attempt.parent/(item['id']+'.invocation.json'),{'argv':command,'partition_note':'Frozen legacy evaluator labels explicit Rust cases development; outer declared manifest preserves Rust validation family. Both arms use identical legacy wrapper.'})
 code=None;error=None
 with (attempt.parent/(item['id']+'.stdout')).open('x') as out,(attempt.parent/(item['id']+'.stderr')).open('x') as err:
  try:code=run_evaluator(command,out,err,960).returncode
  except (OSError,subprocess.TimeoutExpired) as exc:error=type(exc).__name__
 reports=list(attempt.glob('*/report.json'))
 receipt={'declared_attempt':item,'exit_code':code,'error':error,'reports':[str(x.relative_to(root)) for x in reports],'evidence_sha256':file_manifest(attempt) if attempt.exists()else{},'completed_unix':time.time()}
 write_new(attempt.parent/(item['id']+'.finished.json'),receipt);print(json.dumps(receipt),flush=True)
