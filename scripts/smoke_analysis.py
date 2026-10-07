#!/usr/bin/env python3
"""Real HTTP bounded-analysis plumbing with scripted weight-free responses."""
import argparse,json,subprocess,tempfile,threading
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path
class Provider(BaseHTTPRequestHandler):
    requests=[]
    stall_at=None
    stalled=threading.Event()
    release=threading.Event()
    def log_message(self,*_):pass
    def do_POST(self):
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])));self.requests.append(body)
        if len(self.requests)==self.stall_at:
            self.stalled.set();self.release.wait(timeout=10);return
        response=json.dumps({'choices':[{'message':{'role':'assistant','content':'Hypothesis: quoted source contains cedar. No check ran.'},'finish_reason':'stop'}],'usage':{'prompt_tokens':100,'completion_tokens':25}}).encode()
        self.send_response(200);self.send_header('Content-Length',str(len(response)));self.end_headers();self.wfile.write(response)
def main():
    p=argparse.ArgumentParser();p.add_argument('--alt',type=Path,default=Path('target/debug/alt'));a=p.parse_args();alt=a.alt.resolve();server=ThreadingHTTPServer(('127.0.0.1',0),Provider);threading.Thread(target=server.serve_forever,daemon=True).start()
    try:
        with tempfile.TemporaryDirectory(prefix='alt-analysis-') as d:
            root=Path(d);state=root/'state';source=root/'source.txt';source.write_text('noise ☃\n'*3000+'cedar target\n')
            base=[str(alt),'--data-dir',str(state)]
            subprocess.run(base+['init','--model','fixture-no-weights','--uncensored','--endpoint',f'http://127.0.0.1:{server.server_port}/v1'],check=True,capture_output=True)
            result=subprocess.run(base+['analyze',str(source),'Where is the cedar target?','--experimental','--calls','4','--depth','2','--seconds','30','--generated-tokens','256'],capture_output=True,text=True,timeout=40)
            assert result.returncode==0,result.stderr
            report=json.loads(result.stdout);assert report['status']=='analysis-completed-unverified' and report['calls_used']==4 and len(Provider.requests)==4,report
            assert report['cost']['charged_generated_tokens']==100 and report['omitted_source_bytes']>0 and report['demonstrated_retrieval_gap'] is False and report['promoted'] is False,report
            assert len(report['selected_spans'])==2 and len(report['source_observations'])==2 and len(report['synthesis'])==2,report
            assert any('cedar target' in row['messages'][-1]['content'] for row in Provider.requests),Provider.requests
            Provider.requests=[];Provider.stall_at=2
            process=subprocess.Popen(base+['analyze',str(source),'Where is the cedar target?','--experimental','--calls','4','--depth','2','--seconds','3','--generated-tokens','256'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
            try:
                assert Provider.stalled.wait(timeout=5),'Did not reach second actual request'
                saved=[json.loads(p.read_text()) for p in state.glob('evaluations/analysis-*/report.json')]
                partial=next(r for r in saved if r['status']=='running')
                assert partial['calls_used']==1 and len(partial['source_observations'])==1,partial
                stdout,stderr=process.communicate(timeout=8);assert process.returncode!=0,(stdout,stderr)
                timed=json.loads(stdout);assert timed['status']=='timeout' and timed['calls_used']==1,timed
                assert timed['cost']['requests']==2 and timed['cost']['charged_generated_tokens']==256 and timed['cost']['known_generated_tokens'] is None,timed
            finally:
                Provider.release.set()
                if process.poll() is None:process.kill();process.wait()
            print('PASS: recursive levels, retained source spans/partial observations, lexical/Unicode bounds, hard limits, timeout reserved costs and unverified labels (scripted provider; no weights).')
    finally:server.shutdown();server.server_close()
if __name__=='__main__':main()
