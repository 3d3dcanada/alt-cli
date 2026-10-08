#!/usr/bin/env python3
"""Keyboard-only profile/draft/cancellation regressions in an actual PTY.
No model weights: uses local inventory and deterministic delayed ACP fixtures.
"""
import argparse
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import threading
import time
import tomllib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from terminal_harness import Terminal

REPO = Path(__file__).resolve().parents[1]
REQUESTS = []
DELAY = 0
class Inventory(BaseHTTPRequestHandler):
    def log_message(self, *_): pass
    def do_GET(self):
        REQUESTS.append(self.path)
        time.sleep(DELAY)
        raw = json.dumps({'data': [{'id':'aa-first'}, {'id':'fixture'}]}).encode()
        self.send_response(200); self.send_header('Content-Length',str(len(raw))); self.end_headers()
        try: self.wfile.write(raw)
        except (BrokenPipeError, ConnectionResetError): pass

def settle(t, seconds=.25):
    end=time.monotonic()+seconds
    while time.monotonic()<end: t.pump(.02)
def screen(t): settle(t); return '\n'.join(t.screen.display)
def kill(t):
    t.proc.kill();t.proc.wait(timeout=5);os.close(t.master);os.close(t.slave)
def close(t):
    t.send(b'\x11')
    end=time.monotonic()+2
    while t.proc.poll() is None and time.monotonic()<end:
        t.pump(.03)
        if 'Leave Alt?' in '\n'.join(t.screen.display):
            t.send(b'\t\r');break
    t.proc.wait(timeout=10)
    t.close()
def book(state): return json.loads((state/'drafts.json').read_text())
def wait_durable(t,state,text):
    start=time.monotonic();end=start+2
    while time.monotonic()<end:
        t.pump(.02)
        try:
            data=book(state)
            if data['entries'][data['active']]['text']==text: return (time.monotonic()-start)*1000
        except (OSError,ValueError): pass
    raise AssertionError('Draft did not become durable within two seconds')
def palette(t,query):
    t.send(b'\x10');t.wait('Quick actions');t.paste(query);t.send(b'\r')
def configure(binary,state,project,endpoint,engine=None):
    base=[str(binary),'--data-dir',str(state)]
    subprocess.run(base+['init','--model','fixture','--endpoint',endpoint,'--context','4096','--uncensored'],check=True,capture_output=True)
    subprocess.run(base+['inference','--output-tokens','256','--action-headroom','128','--temperature','0.35','--requests','7','--generated-tokens','2048'],check=True,capture_output=True)
    (state/'preferences.toml').write_text('mouse=false\nproject='+json.dumps(str(project))+'\n'+('engine_path='+json.dumps(str(engine))+'\n' if engine else ''))
    return base

def main():
    global DELAY
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--alt',type=Path,default=REPO/'target/debug/alt')
    parser.add_argument('--output',type=Path)
    args=parser.parse_args();binary=args.alt.resolve();rows=[]
    server=ThreadingHTTPServer(('127.0.0.1',0),Inventory);threading.Thread(target=server.serve_forever,daemon=True).start()
    endpoint=f'http://127.0.0.1:{server.server_port}/v1'
    try:
        for width,height in [(80,24),(60,18)]:
            with tempfile.TemporaryDirectory(prefix='alt-draft-ui-') as directory:
                root=Path(directory);project=root/'project';project.mkdir();state=root/'state'
                configure(binary,state,project,endpoint)
                before=tomllib.loads((state/'config.toml').read_text())
                t=Terminal(binary,state,project,width,height)
                try:
                    t.wait('Start a new conversation');t.send(b'\x1b4');t.wait('Saved connections');t.send(b'e');t.wait('Server address')
                    assert 'Enter model ID' in screen(t),'Manual model entry must be visible at the supported size'
                    t.send(b'\r');t.wait('Choose your model');t.send(b'\r');t.wait('Connection saved')
                    assert before==tomllib.loads((state/'config.toml').read_text()),'Editing connection reset fields or selected model'
                    t.send(b'\x1b4');t.wait('Saved connections');t.send(b'e');t.wait('Server address');count=len(REQUESTS)
                    t.send(b'\x0f');t.wait('Enter a model identifier');assert len(REQUESTS)==count
                    t.send(b'\r');t.wait('Connection saved');assert before==tomllib.loads((state/'config.toml').read_text())
                    # Network cancellation finishes promptly and cannot reopen the picker.
                    DELAY=2;t.send(b'\x1b4');t.wait('Saved connections');t.send(b'e');t.wait('Server address');t.send(b'\r');t.wait('Checking the connection')
                    start=time.monotonic();t.send(b'\x1b');t.wait('Operation cancelled',timeout=1);cancel_ms=(time.monotonic()-start)*1000
                    settle(t,2.2);assert 'Choose your model' not in screen(t);DELAY=0
                    # Durable restart and explicit New preserve drafts without sending.
                    t.send(b'\x1b2');t.wait('Describe what you want');draft='UNSENT café 日本語'
                    draft_started=time.monotonic();t.paste(draft);t.wait(draft);wait_durable(t,state,draft);durable_ms=(time.monotonic()-draft_started)*1000
                    kill(t);t=Terminal(binary,state,project,width,height);t.wait('Draft restored');t.send(b'\x1b2');t.wait(draft)
                    t.send(b'\x0e');t.wait('New conversation');t.send(b'\x04');t.wait('Saved drafts');t.send(b'\r');t.wait('Draft restored into this project');t.wait(draft)
                    # Large paste fails visibly and does not replace the existing draft.
                    t.paste('x'*(64*1024));t.wait('64 KiB');t.send(b'\x1b');t.wait(draft)
                    palette(t,'save drafts');t.wait('Drafts saved to local storage');t.close()
                    rows.append({'size':f'{width}x{height}','profile_preserved':True,'manual_id_no_inventory':True,'cancel_ms':round(cancel_ms,2),'input_to_durable_ms':round(durable_ms,2),'abrupt_restore':True,'new_restore':True,'oversized_paste_feedback':True,'keyboard_only':True})
                finally:
                    DELAY=0
                    if t.proc.poll() is None: t.close()
        # A second TUI must refuse the same draft store, and failed saves must
        # keep the editor open until storage is repaired and explicitly flushed.
        with tempfile.TemporaryDirectory(prefix='alt-draft-storage-ui-') as directory:
            root=Path(directory);project=root/'project';project.mkdir();state=root/'state'
            configure(binary,state,project,endpoint)
            t=Terminal(binary,state,project,60,18)
            try:
                t.wait('Start a new conversation');t.send(b'\x1b2');t.paste('STORAGE FAILURE RECOVERY');wait_durable(t,state,'STORAGE FAILURE RECOVERY')
                other=Terminal(binary,state,project,60,18)
                try:
                    other.wait('Another Alt window owns drafts');other.proc.wait(timeout=5);assert other.proc.returncode!=0
                finally:
                    if other.proc.poll() is None:other.proc.kill();other.proc.wait(timeout=5)
                    os.close(other.master);os.close(other.slave)
                saved=book(state);(state/'drafts.json').unlink();(state/'drafts.json').mkdir()
                t.paste(' PLUS UNSAVED');t.wait('Draft not saved');t.send(b'\x1b');t.send(b'\x11');t.wait('Could not save drafts');assert t.proc.poll() is None
                (state/'drafts.json').rmdir();t.send(b'\x1b');palette(t,'save drafts');t.wait('Drafts saved to local storage');close(t)
                assert book(state)['entries'][saved['active']]['text']=='STORAGE FAILURE RECOVERY PLUS UNSAVED'
                rows.append({'size':'60x18','second_window_refused':True,'failed_quit_keeps_editor':True,'repair_then_flush_preserves_text':True})
            finally:
                if t.proc.poll() is None:t.proc.terminate();t.proc.wait(timeout=10)
        # Connection success/failure/cancellation must preserve the independently edited next draft.
        for mode in ['success','failure','cancel']:
            with tempfile.TemporaryDirectory(prefix='alt-submit-ui-') as directory:
                root=Path(directory);project=root/'project';project.mkdir();state=root/'state'
                engine=root/'engine.py'
                source=(REPO/'tests/fixtures/acp_engine.py').read_text().replace('import sys','import sys\nimport time')
                inject='\n        time.sleep(1.5)' + ('\n        sys.exit(17)' if mode=='failure' else '')
                engine.write_text(source.replace('if method == "initialize":','if method == "initialize":'+inject));engine.chmod(0o755)
                configure(binary,state,project,endpoint,engine)
                t=Terminal(binary,state,project,80,24)
                try:
                    t.wait('Start a new conversation');t.send(b'\x1b2');t.wait('Describe what you want');t.paste('ORIGINAL REQUEST');t.send(b'\r');t.wait('Connecting')
                    t.paste('NEXT REQUIREMENT');t.wait('NEXT REQUIREMENT')
                    if mode=='success':
                        t.wait('Response ready');t.wait('NEXT REQUIREMENT')
                        with sqlite3.connect(state/'sessions.db') as db: events=[json.loads(r[0]) for r in db.execute('SELECT payload FROM events')]
                        messages=[e for e in events if e.get('type')=='user']
                        assert len(messages)==1 and messages[0]['text']=='ORIGINAL REQUEST' and messages[0].get('message_id')
                    elif mode=='failure':
                        t.wait("Let's get this working");t.send(b'\x1b');t.wait('NEXT REQUIREMENT')
                    else:
                        t.send(b'\x1b');t.wait('Connection cancelled');t.wait('NEXT REQUIREMENT')
                    close(t);saved=book(state);current=saved['entries'][saved['active']]
                    assert current['text']=='NEXT REQUIREMENT'
                    if mode!='success': assert current['pending']['text']=='ORIGINAL REQUEST' and current['pending']['id']
                    t=Terminal(binary,state,project,80,24);t.wait('Draft restored');t.send(b'\x1b2');t.wait('NEXT REQUIREMENT')
                    if mode!='success':
                        t.send(b'\x04');t.wait('Restore interrupted submission');t.send(b'\r');t.wait('Submission restored');t.wait('ORIGINAL REQUEST')
                        assert any(d['text']=='NEXT REQUIREMENT' for d in book(state)['entries'].values())
                    t.close();print('PASS startup '+mode,flush=True);rows.append({'startup':mode,'next_draft_preserved':True,'submission_id_recorded_or_recoverable':True,'orderly_restart':True})
                finally:
                    if t.proc.poll() is None:t.close()
        print(json.dumps({'passed':True,'attempts':rows,'scope':'Actual keyboard PTYs with deterministic inventory/ACP; no model quality or human novice measurement'},indent=2),flush=True)
        if args.output:
            args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps({'passed':True,'attempts':rows},indent=2)+'\n')
    finally:server.shutdown();server.server_close()
if __name__=='__main__':main()
