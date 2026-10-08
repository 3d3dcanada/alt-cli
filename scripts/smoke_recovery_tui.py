#!/usr/bin/env python3
"""Keyboard-only recovery and evidence journeys, without model weights.
Checks persisted state independently of UI labels and exercises minimum-size forms.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sqlite3
import tempfile
import time
from terminal_harness import Terminal
from smoke_draft_tui import palette, close, settle
REPO=Path(__file__).resolve().parents[1]

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--alt',type=Path,default=Path(os.environ.get('ALT_TEST_BINARY',REPO/'target/debug/alt')))
    parser.add_argument('--output',type=Path)
    args=parser.parse_args();binary=args.alt.resolve();rows=[]
    for width,height in [(80,24),(60,18)]:
        with tempfile.TemporaryDirectory(prefix='alt-recovery-ui-') as directory:
            root=Path(directory);project=root/'project';project.mkdir();state=root/'state';state.mkdir()
            external=root/'external.txt';external.write_text('external input')
            (project/'original.txt').write_text('ORIGINAL FILE VERSION\n')
            (project/'check.py').write_text("import sys\nprint('CHECK_STDOUT')\nprint('LATE_STDERR_EVIDENCE',file=sys.stderr)\n")
            (state/'preferences.toml').write_text('project='+json.dumps(str(project))+'\nmouse=false\naccess_policy="trusted"\n')
            base=[str(binary),'--data-dir',str(state)]
            def cli(*items):return subprocess.check_output(base+list(items),cwd=project,text=True)
            cli('task','configure-check','audit','--timeout','23','--kind','build','--','python3','check.py')
            cli('task','check','audit')
            before=json.loads(cli('task','checks'))['configured'][0]
            term=Terminal(binary,state,project,width,height)
            try:
                term.wait('Connect your first model');palette(term,'Retained check output');term.wait('Retained check output');term.send(b'\x1b[B\r');term.wait('Retained stderr');term.wait('LATE_STDERR_EVIDENCE');term.send(b'\r');term.wait('Execution output navigation');term.send(b'\r');term.wait('Retained stdout');term.wait('CHECK_STDOUT');term.send(b'\r');term.send(b'\x1b')
                palette(term,'Check inputs and generated files');term.wait('Choose a check');term.send(b'\r');term.wait('Check inputs and generated files');term.send(b'\r');term.wait('External input files');term.paste(str(external));term.send(b'\x13');term.wait('Save these check inputs?');term.send(b'\t\r');term.wait('Check input declaration saved')
                palette(term,'Check inputs and generated files');term.wait('Choose a check');term.send(b'\r');term.wait('Check inputs and generated files');term.send(b'\x1b[B\r');term.wait('Generated output paths');term.paste('coverage/\nreports/');term.send(b'\x13');term.wait('Save these check inputs?');term.send(b'\t\r');term.wait('Check input declaration saved')
                after=json.loads(cli('task','checks'))['configured'][0]
                assert after['contract']['inputs']=={'files':[str(external)],'generated':['coverage/','reports/']},after
                assert after['argv']==before['argv'] and after['timeout_secs']==23 and after['contract']['kind']=='build'
                palette(term,'Evidence storage budget');term.wait('Inference evidence warning');term.send(b'\x15');term.paste('64');term.send(b'\r');term.wait('Evidence warning budget saved')
                assert json.loads(cli('state','usage'))['inference_warning_bytes']==64*1024*1024
                term.send(b'\x1b9');term.wait('Project files');term.send(b'/');term.wait('Filter project files');term.paste('original.txt');term.send(b'\r');term.send(b'e');term.wait('Edit original.txt');term.send(b'\x15');term.paste('REPLACEMENT FILE VERSION\n');term.send(b'\x13');term.wait('Save original.txt?');term.send(b'\t\r');term.wait('File saved with an undo checkpoint');assert (project/'original.txt').read_text()=='REPLACEMENT FILE VERSION\n'
                palette(term,'Recovered file versions');term.wait('Recovered file versions');term.send(b'\r');term.wait('Restore this recovered file');assert (project/'original.txt').read_text()=='REPLACEMENT FILE VERSION\n';term.send(b'\t\r');term.wait('File saved with an undo checkpoint');assert (project/'original.txt').read_text()=='ORIGINAL FILE VERSION\n'
                close(term);rows.append({'size':f'{width}x{height}','raw_stream_switch':True,'declared_inputs_preserve_contract':True,'storage_budget_persisted':True,'recovered_preview_and_restore':True,'keyboard_only':True})
            finally:
                if term.proc.poll() is None:term.proc.terminate();term.proc.wait(timeout=10)
    # Explicit correction retires only the chosen request, keeping exact history.
    with tempfile.TemporaryDirectory(prefix='alt-requirement-ui-') as directory:
        root=Path(directory);project=root/'project';project.mkdir();state=root/'state'
        engine=REPO/'tests/fixtures/acp_engine.py'
        base=[str(binary),'--data-dir',str(state)]
        subprocess.run(base+['init','--model','fixture','--endpoint','http://127.0.0.1:1/v1','--uncensored'],check=True,capture_output=True)
        (state/'preferences.toml').write_text('mouse=false\nproject='+json.dumps(str(project))+'\nengine_path='+json.dumps(str(engine))+'\n')
        term=Terminal(binary,state,project,60,18)
        try:
            term.wait('Start a new conversation');term.send(b'\x1b2');term.paste('Use the original public API.');term.send(b'\r');term.wait('Response ready')
            palette(term,'What Alt currently understands');term.wait('What Alt currently understands');term.wait('Use the original public API.');term.send(b'\r')
            palette(term,'Correct a requirement');term.wait('Choose a requirement to correct');term.send(b'\r');term.wait('Replace this user requirement');term.send(b'\x15');term.paste('Use public API version two.');term.send(b'\x13');term.wait('Why is this requirement changing?');term.paste('The compatibility target changed.');term.send(b'\r');term.wait('Replace this requirement?');term.send(b'\t\r');term.wait('Requirement corrected')
            history=json.loads(subprocess.check_output(base+['task','requests'],cwd=project,text=True))
            with sqlite3.connect(state/'sessions.db') as db: events=[json.loads(r[0]) for r in db.execute('SELECT payload FROM events')]
            assert len([e for e in events if e.get('type')=='user'])==1,'Correction must not send a model prompt'
            serialized=json.dumps(history)
            assert 'Use the original public API.' in serialized and 'Use public API version two.' in serialized and 'The compatibility target changed.' in serialized,history
            assert 'superseded_by' in serialized
            close(term);rows.append({'size':'60x18','explicit_requirement_correction':True,'earlier_text_retained':True,'no_model_prompt_for_correction':True})
        finally:
            if term.proc.poll() is None:term.proc.terminate();term.proc.wait(timeout=10)
    result={'passed':True,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'attempts':rows,'scope':'Actual keyboard PTYs, deterministic ACP and local check commands; no model quality or human novice measurement'}
    print(json.dumps(result,indent=2),flush=True)
    if args.output:args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(result,indent=2)+'\n')
if __name__=='__main__':main()
