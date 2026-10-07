#!/usr/bin/env python3
"""Real Goose + deterministic provider: handles, serial copies, checks, promotion, undo.

Fixture responses are scripted and are not a live-model success claim.
"""
import argparse,json,re,signal,subprocess,tempfile,threading,time
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path
class Provider(BaseHTTPRequestHandler):
    stall=False
    stalled=threading.Event()
    release=threading.Event()
    def log_message(self,*args):pass
    def do_POST(self):
        request=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        if Provider.stall:
            Provider.stall=False;Provider.stalled.set();Provider.release.wait(timeout=30)
            return
        tools={t['function']['name'].split('__')[-1]:t['function']['name'] for t in request.get('tools',[])}
        messages=request['messages'];prior=[m for m in messages if m['role']=='tool'];step=len(prior)
        if step==0:name='read';arguments={'path':'value.txt'}
        elif step==1:name='remember';arguments={'kind':'plan','text':'Replace the current range; run unchanged behavior check'}
        elif step==2:
            content=prior[0]['content'];content=json.dumps(content) if not isinstance(content,str) else content
            match=re.search(r'Range handle: (v1:[a-f0-9]+:\d+:\d+:[a-f0-9]+)',content);assert match,content
            name='edit';arguments={'path':'value.txt','operation':'handle','handle':match[1],'new_text':'fixed ☃\n','reason':'Repair fixture value'}
        elif step==3:name='run_check';arguments={'name':'acceptance'}
        else:name=None
        if name:message={'role':'assistant','content':None,'tool_calls':[{'id':f'call-{step}','type':'function','function':{'name':tools[name],'arguments':json.dumps(arguments)}}]};finish='tool_calls'
        else:message={'role':'assistant','content':'Recorded check evidence; review its scope.'};finish='stop'
        self.send_response(200);self.send_header('Content-Type','text/event-stream');self.end_headers()
        if 'tool_calls' in message:message['tool_calls'][0]['index']=0
        for row in [{'choices':[{'index':0,'delta':message,'finish_reason':None}]},{'choices':[{'index':0,'delta':{},'finish_reason':finish}],'usage':{'prompt_tokens':100,'completion_tokens':25,'total_tokens':125}}]:self.wfile.write(('data: '+json.dumps({'id':'fixture','object':'chat.completion.chunk','created':1,'model':'fixture',**row})+'\n\n').encode())
        self.wfile.write(b'data: [DONE]\n\n')

def main():
    p=argparse.ArgumentParser();p.add_argument('--alt',type=Path,default=Path('target/debug/alt'));p.add_argument('--goose',type=Path,required=True);a=p.parse_args();alt=a.alt.resolve();goose=a.goose.resolve();server=ThreadingHTTPServer(('127.0.0.1',0),Provider);threading.Thread(target=server.serve_forever,daemon=True).start()
    try:
        with tempfile.TemporaryDirectory(prefix='alt-candidates-') as d:
            root=Path(d);workspace=root/'project';workspace.mkdir();state=root/'state';(workspace/'value.txt').write_text('broken\n')
            assertion=root/'assertion.py';assertion.write_text("import json,os\nfrom pathlib import Path\nassert Path('value.txt').read_text()=='fixed ☃\\n'\nPath(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'value behavior','status':'passed'}]}))\n")
            def run(*args,ok=True):
                result=subprocess.run([str(alt),'--data-dir',str(state),'--engine',str(goose),'--access','trusted',*args],cwd=workspace,capture_output=True,text=True,timeout=90)
                if ok:assert result.returncode==0,(args,result.stderr,result.stdout[-4000:])
                return result
            run('init','--model','fixture','--uncensored','--endpoint',f'http://127.0.0.1:{server.server_port}/v1')
            run('task','configure-check','acceptance','--','python3',str(assertion));run('task','contract','acceptance','--kind','tests','--format','json','--report','.report.json','--assertion',str(assertion));run('task','require','value','--check','acceptance','--description','Value fixed')
            manifest=json.loads(run('candidates','run','Repair value.txt','--effort','careful','--seconds','60','--generated-tokens','1000','--requests','12').stdout)
            assert manifest['selected']==1 and len(manifest['rows'])==1,manifest
            assert manifest['spent_generated_tokens']==125 and manifest['spent_requests']==5,manifest
            assert (workspace/'value.txt').read_text()=='broken\n','Candidate changed original before promotion'
            assert manifest['rows'][0]['verification']['behavioral_acceptance'] is True
            (workspace/'user-note.txt').write_text('preserve me')
            rejected=run('candidates','apply',manifest['id'],'1',ok=False);assert rejected.returncode!=0 and 'project changed' in rejected.stderr.lower()
            assert (workspace/'value.txt').read_text()=='broken\n';(workspace/'user-note.txt').unlink()
            promoted=json.loads(run('candidates','apply',manifest['id'],'1').stdout);assert (workspace/'value.txt').read_text()=='fixed ☃\n'
            run('task','undo',promoted['applied_checkpoints'][0]);assert (workspace/'value.txt').read_text()=='broken\n'
            Provider.stall=True
            command=[str(alt),'--data-dir',str(state),'--engine',str(goose),'--access','trusted','candidates','run','Repair value.txt','--effort','careful','--seconds','60','--generated-tokens','1000','--requests','12']
            interrupted=subprocess.Popen(command,cwd=workspace,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
            try:
                assert Provider.stalled.wait(timeout=15),'Candidate never reached the actual provider'
                interrupted.send_signal(signal.SIGINT)
                stdout,stderr=interrupted.communicate(timeout=15)
                assert interrupted.returncode==0,(stdout,stderr)
            finally:
                Provider.release.set()
                if interrupted.poll() is None:interrupted.kill();interrupted.wait()
            cancelled=json.loads(stdout)
            assert cancelled['status']=='cancelled' and cancelled['selected'] is None,cancelled
            assert cancelled['spent_generated_tokens']==500 and cancelled['spent_requests']==1,cancelled
            assert (workspace/'value.txt').read_text()=='broken\n'
            saved_copy=Path(cancelled['rows'][0]['source_directory'])
            assert (saved_copy/'value.txt').exists()
            resumed=json.loads(run('candidates','run','Repair value.txt','--effort','careful','--seconds','60','--generated-tokens','1000','--requests','12','--resume',cancelled['id']).stdout)
            assert resumed['selected']==2 and len(resumed['rows'])==2,resumed
            assert resumed['spent_generated_tokens']==625 and resumed['spent_requests']==6,resumed
            assert (saved_copy/'value.txt').exists() and (workspace/'value.txt').read_text()=='broken\n'
            print('PASS: actual Goose native handles, independent selection, shared costs, source protection, promotion conflict/undo, cancellation reservation and resume without a free retry.')
    finally:server.shutdown();server.server_close()

if __name__=='__main__':main()
