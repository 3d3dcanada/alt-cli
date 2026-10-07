#!/usr/bin/env python3
"""Real Goose streaming, disconnect/cancel/crash, persisted history and reconnect.

The HTTP provider is deterministic and has no weights. This measures application
recovery, never model intelligence. Every subprocess has a bounded lifetime.
"""
import argparse, ctypes, hashlib, json, os, signal, subprocess, tempfile, threading, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',type=Path,default=Path('target/debug/alt'))
p.add_argument('--engine',type=Path,required=True)
p.add_argument('--rounds',type=int,default=12)
p.add_argument('--output',type=Path,required=True)
a=p.parse_args(); binary=a.binary.resolve(); assert 1<=a.rounds<=200
assert ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)==0
mode='normal'; started=threading.Event(); rows=[]

class Provider(BaseHTTPRequestHandler):
    def log_message(self,*_): pass
    def do_GET(self):
        body=json.dumps({'data':[{'id':'stream-fixture'}]}).encode()
        self.send_response(200); self.send_header('Content-Type','application/json'); self.send_header('Content-Length',str(len(body))); self.end_headers(); self.wfile.write(body)
    def do_POST(self):
        request=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        assert request['model']=='stream-fixture'
        selected=mode
        self.send_response(200); self.send_header('Content-Type','text/event-stream')
        if selected=='disconnect': self.send_header('Content-Length','9999999')
        self.end_headers()
        try:
            for n in range(400):
                chunk={'id':'stream-fixture','object':'chat.completion.chunk','created':1,'model':'stream-fixture','choices':[{'index':0,'delta':{'content':f'STREAM_RECEIPT_{n} '},'finish_reason':None}]}
                self.wfile.write(('data: '+json.dumps(chunk)+'\n\n').encode()); self.wfile.flush(); started.set()
                if selected=='disconnect' and n==2: return
                time.sleep(.005)
            chunk['choices']=[{'index':0,'delta':{},'finish_reason':'stop'}]
            self.wfile.write(('data: '+json.dumps(chunk)+'\n\ndata: [DONE]\n\n').encode()); self.wfile.flush()
        except (BrokenPipeError,ConnectionResetError): pass

server=ThreadingHTTPServer(('127.0.0.1',0),Provider); threading.Thread(target=server.serve_forever,daemon=True).start()
def descendants(pid):
    parents={}; identities={}
    for path in Path('/proc').iterdir():
        if not path.name.isdigit(): continue
        try:
            fields=(path/'stat').read_text().rsplit(') ',1)[1].split()
            parents[int(path.name)]=int(fields[1]); identities[int(path.name)]=(int(path.name),fields[19])
        except (FileNotFoundError,ProcessLookupError,PermissionError): pass
    seen={pid}; frontier={pid}
    while frontier:
        frontier={child for child,parent in parents.items() if parent in frontier}-seen
        seen.update(frontier)
    return [identities[child] for child in seen if child!=pid and child in identities]
def exists(identity):
    try: return Path(f'/proc/{identity[0]}/stat').read_text().rsplit(') ',1)[1].split()[19]==identity[1]
    except FileNotFoundError: return False
def reap():
    while True:
        try:
            if os.waitpid(-1,os.WNOHANG)[0]==0: break
        except ChildProcessError: break

try:
    with tempfile.TemporaryDirectory(prefix='alt-stream-recovery-') as d:
        root=Path(d); state=root/'state'; project=root/'project'; project.mkdir()
        base=[str(binary),'--data-dir',str(state),'--engine',str(a.engine.resolve())]
        subprocess.run(base+['init','--model','stream-fixture','--endpoint',f'http://127.0.0.1:{server.server_port}/v1'],cwd=project,check=True,capture_output=True)
        session=None
        for round in range(a.rounds):
            for selected in ['normal','disconnect','cancel','engine-crash','reconnect']:
                mode=selected; started.clear(); begin=time.monotonic(); row={'round':round+1,'mode':selected,'passed':False}; proc=None
                try:
                    command=base+['run',f'Stream recovery round {round+1} {selected}','--json','--timeout','30']
                    if session: command+=['--resume',session]
                    evidence=a.output.resolve().parent/'stream-evidence'/f'{round+1}-{selected}'
                    evidence.mkdir(parents=True,exist_ok=True)
                    out=evidence/'turn.jsonl'; err=evidence/'turn.stderr'
                    with out.open('w') as stdout,err.open('w') as stderr:
                        proc=subprocess.Popen(command,cwd=project,stdout=stdout,stderr=stderr,start_new_session=True)
                        assert started.wait(20), 'No provider stream'
                        # Wait for actual forwarding, not just an open HTTP connection.
                        deadline=time.monotonic()+10
                        while 'STREAM_RECEIPT_0' not in out.read_text() and proc.poll() is None and time.monotonic()<deadline: time.sleep(.02)
                        assert 'STREAM_RECEIPT_0' in out.read_text(), 'First streamed content was lost'
                        owned=descendants(proc.pid); assert owned, 'No owned engine to test'
                        if selected=='cancel': proc.send_signal(signal.SIGINT)
                        elif selected=='engine-crash': os.kill(next(identity[0] for identity in owned if Path(f'/proc/{identity[0]}/comm').read_text().strip()=='goose'),signal.SIGKILL)
                        proc.wait(timeout=40)
                    events=[json.loads(line) for line in out.read_text().splitlines()]
                    session=next(e['session']['id'] for e in events if e.get('type')=='session')
                    if selected in ['normal','reconnect']:
                        assert proc.returncode==0 and 'STREAM_RECEIPT_399' in out.read_text(), 'Complete stream missing'
                    elif selected in ['disconnect','engine-crash']:
                        assert proc.returncode!=0, 'Broken stream reported clean completion'
                    exported=subprocess.check_output(base+['export',session],cwd=project,text=True,timeout=15)
                    assert 'STREAM_RECEIPT_0' in exported and f'Stream recovery round {round+1}' in exported, 'Persisted history missing'
                    deadline=time.monotonic()+3
                    while True:
                        reap(); remaining=[identity for identity in owned if exists(identity)]
                        if not remaining or time.monotonic()>=deadline: break
                        time.sleep(.03)
                    assert not remaining, f'Owned descendants leaked: {remaining}'
                    row.update(passed=True,exit=proc.returncode,events=len(events),retained_state_bytes=sum(f.stat().st_size for f in state.rglob('*') if f.is_file()),owned_processes_reaped=len(owned))
                except Exception as error: row['error']=str(error)
                finally:
                    if proc and proc.poll() is None: os.killpg(proc.pid,signal.SIGKILL); proc.wait()
                    row['seconds']=round_seconds=time.monotonic()-begin; rows.append(row)
                    report={'schema':1,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'expected':a.rounds*5,'attempts':rows,'passed':len(rows)==a.rounds*5 and all(r['passed'] for r in rows),'scope':'Real Alt and Goose with a deterministic streaming provider; no model weights, no physical GPU or human usability measurement.'}
                    a.output.parent.mkdir(parents=True,exist_ok=True); a.output.write_text(json.dumps(report,indent=2)+'\n')
                    print(f'{round+1} {selected}: {row["passed"]} {round_seconds:.2f}s',flush=True)
                if not row['passed']: raise SystemExit(1)
finally: server.shutdown(); server.server_close()
