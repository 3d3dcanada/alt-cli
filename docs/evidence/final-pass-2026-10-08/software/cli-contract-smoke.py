from pathlib import Path
import tempfile,subprocess,json,hashlib,os
binary=Path('/workspace/alt-cli/target/portable-glibc231/release/alt')
rows=[]
with tempfile.TemporaryDirectory(prefix='alt-final-cli-',dir='/workspace') as tmp:
 root=Path(tmp);project=root/'project';project.mkdir();state=root/'state'
 (project/'answer.txt').write_text('42')
 external=root/'expected.txt';external.write_text('42')
 assertion=root/'assertion.py';assertion.write_text('import json,os\nfrom pathlib import Path\nassert Path("answer.txt").read_text()==Path('+repr(str(external))+').read_text()\nPath("output.txt").write_text("declared generated output")\nprint("FULL_RAW_STDOUT")\nPath(os.environ["ALT_CHECK_REPORT"]).write_text(json.dumps({"schema":1,"complete":True,"tests":[{"name":"actual answer","status":"passed"}]}))\n')
 base=[str(binary),'--data-dir',str(state),'--access','trusted']
 def run(args,code=0):
  r=subprocess.run(base+args,cwd=project,text=True,capture_output=True,timeout=30)
  rows.append({'args':args,'exit_code':r.returncode,'stdout':r.stdout,'stderr':r.stderr})
  assert r.returncode==code,(args,r.returncode,r.stderr)
  return json.loads(r.stdout)
 cfg=run(['task','configure-check','answer','--timeout','33','--kind','tests','--format','json','--report','.alt-report.json','--assertion',str(assertion),'--input-file',str(external),'--generated-output','output.txt','--','python3','{assertion}'])
 updated=run(['task','configure-check','answer','--','python3','{assertion}'])
 assert cfg==updated,(cfg,updated)
 run(['task','require','answer','--check','answer'])
 check=run(['task','check','answer']);assert not check['execution']['immutable_inputs']
 verified=run(['task','verify']);assert verified['behavioral_acceptance']
 log=run(['task','logs',check['id'],'--stream','stdout','--limit','1000']);assert 'FULL_RAW_STDOUT' in json.dumps(log)
 req=run(['task','requests']);seq=req['active'][0]['seq']
 correction=run(['task','correct-request',str(seq),'Keep the exact answer and spacing.','--reason','Explicit CLI correction'])
 assert correction['active'][0]['body']=='Keep the exact answer and spacing.'
 exported=run(['task','export']);assert 'Explicit CLI correction' in json.dumps(exported)
 external.write_text('43');stale=run(['task','verify'],code=1);assert not stale['complete']
 run(['state','budget','--inference-mib','512'])
report={'passed':True,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'scope':'Disposable real CLI workflow, no model weights','rows':rows}
Path('/workspace/.alt-final-validation/cli-contract-smoke.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: atomic contract/config preservation, generated/external inputs, raw logs, exact correction/export and external-input stale evidence')
