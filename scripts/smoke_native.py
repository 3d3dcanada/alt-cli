#!/usr/bin/env python3
"""Actual native-probe HTTP fixtures: predicting the input cannot pass result use."""
import argparse,json,re,subprocess,tempfile,threading
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path

class Provider(BaseHTTPRequestHandler):
    use_result=False
    requests=[]
    nonce=None
    def log_message(self,*_):pass
    def do_POST(self):
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])));Provider.requests.append(body)
        results=[m for m in body['messages'] if m['role']=='tool']
        if not results:
            Provider.nonce=re.search(r'text exactly (ALT_NATIVE_[a-f0-9-]+)',body['messages'][0]['content'])[1]
            message={'role':'assistant','content':None,'tool_calls':[{'index':0,'id':'actual-fixture-call','type':'function','function':{'name':'echo_probe','arguments':json.dumps({'text':Provider.nonce})}}]};finish='tool_calls'
        else:
            returned=results[-1]['content'];assert returned!=Provider.nonce and returned not in json.dumps(Provider.requests[0])
            message={'role':'assistant','content':returned if Provider.use_result else Provider.nonce};finish='stop'
        self.send_response(200);self.send_header('Content-Type','text/event-stream');self.end_headers()
        self.wfile.write(('data: '+json.dumps({'choices':[{'index':0,'delta':message,'finish_reason':finish}],'usage':{'completion_tokens':25}})+'\n\ndata: [DONE]\n\n').encode())

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--alt',type=Path,default=Path('target/debug/alt'));p.add_argument('--output',type=Path);a=p.parse_args();alt=a.alt.resolve();rows=[]
    server=ThreadingHTTPServer(('127.0.0.1',0),Provider);threading.Thread(target=server.serve_forever,daemon=True).start()
    try:
        with tempfile.TemporaryDirectory(prefix='alt-native-contract-') as directory:
            state=Path(directory)/'state';base=[str(alt),'--data-dir',str(state)]
            subprocess.run(base+['init','--model','fixture-no-weights','--uncensored','--endpoint',f'http://127.0.0.1:{server.server_port}/v1'],check=True,capture_output=True)
            for uses_result in (False,True):
                Provider.use_result=uses_result;Provider.requests=[]
                result=subprocess.run(base+['qualify-tools','--timeout','10'],capture_output=True,text=True,timeout=15);report=json.loads(result.stdout)
                assert report['challenge_version']==2 and len(Provider.requests)==2,report
                assert (result.returncode==0)==uses_result and (report['status']=='passed')==uses_result,report
                if not uses_result:assert 'actual host result' in report['error']
                rows.append({'uses_result':uses_result,'report':report,'requests':Provider.requests})
        if a.output:a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps({'scope':'Real native HTTP round trips with scripted weight-free responses; not model quality','rows':rows},indent=2)+'\n')
        print('PASS: repeated input fails; a newly generated host result is consumed; actual requests and native calls are retained.')
    finally:server.shutdown();server.server_close()

if __name__=='__main__':main()
