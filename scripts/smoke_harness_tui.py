#!/usr/bin/env python3
"""New small-model settings and effort choices in a real PTY; no model inference."""
import argparse,json,subprocess,tempfile,time,tomllib
from pathlib import Path
from terminal_harness import Terminal

def main():
    p=argparse.ArgumentParser();p.add_argument('--alt',type=Path,default=Path('target/debug/alt'));p.add_argument('--width',type=int,default=120);p.add_argument('--height',type=int,default=40);a=p.parse_args();binary=a.alt.resolve()
    with tempfile.TemporaryDirectory(prefix='alt-harness-tui-') as d:
        root=Path(d);project=root/'project';project.mkdir();state=root/'state'
        subprocess.run([str(binary),'--data-dir',str(state),'init','--model','fixture-no-weights','--uncensored'],check=True,capture_output=True)
        (state/'preferences.toml').write_text('project='+json.dumps(str(project))+'\n')
        term=Terminal(binary,state,project,width=a.width,height=a.height)
        def choose(index):term.send(b'\x1b[B'*index+b'\r')
        def wait_words(text):
            deadline=time.monotonic()+10
            while time.monotonic()<deadline:
                term.pump()
                # Border glyphs separate wrapped terminal rows; keep every word.
                visible=' '.join(' '.join(line[line.find('│')+1:line.rfind('│')].replace('│',' ') for line in term.screen.display if line.count('│')>=2).split())
                if text in visible:return
            raise AssertionError('Missing wrapped text '+repr(text)+'\n'+'\n'.join(term.screen.display))
        def settings():term.send(b'\x1b6');term.wait('Preferences');term.send(b'\x1b[A'*30);choose(13);term.wait('Model and runtime settings')
        def preferences():return tomllib.loads((state/'preferences.toml').read_text())
        def allocation():return next(iter(tomllib.loads((state/'config.toml').read_text())['profiles'].values()))['inference']
        try:
            term.wait('Alt');settings();choose(5);term.wait('Model output and sampling');choose(0);term.wait('Set Total output tokens');term.send(b'\x15');term.paste('600');term.send(b'\r');term.wait('Model allocation saved');assert allocation()['output_tokens']==600
            settings();choose(5);term.wait('Model output and sampling');choose(3);term.wait('Set Temperature');term.send(b'\x15');term.paste('0.25');term.send(b'\r');term.wait('Model allocation saved');assert allocation()['temperature']==0.25
            settings();choose(5);term.wait('Model output and sampling');choose(3);term.wait('Set Temperature');term.send(b'\x15\r');term.wait('Model allocation saved');assert 'temperature' not in allocation()
            settings();choose(4);term.wait('Task workflow');choose(1);term.wait('Task workflow saved');assert preferences()['workflow']=='host'
            settings();choose(3);term.wait('Choose a task skill');choose(1);term.wait('Skill choice saved');assert preferences()['active_skill']=='python-repair'
            term.send(b'\x1b6');term.wait('Preferences');term.send(b'\x1b[A'*30);choose(15);term.wait('Which tools fit this task?');choose(5);term.wait('Tool focus saved');assert preferences()['tool_profile']=='compact-lines'
            term.send(b'\x1b6');term.wait('Preferences');term.send(b'\x1b[A'*30);choose(15);term.wait('Which tools fit this task?');choose(4);term.wait('Tool focus saved');assert preferences()['tool_profile']=='compact'
            settings();choose(0);term.wait('Reviewed instruction improvements');wait_words('Activation requires');term.send(b'\x1b')
            settings();choose(1);term.wait('Explore independently checked candidates');term.paste('Repair the behavior');term.send(b'\r');term.wait('Choose effort');term.wait('Thorough');choose(2);term.wait('Shared candidate allowance');term.wait('Standard');term.wait('Extended');choose(3);term.wait('Shared time in seconds');term.send(b'\x15');term.paste('120');term.send(b'\r');term.wait('Shared generated tokens');term.send(b'\x1b');assert not (state/'candidates').exists()
            term.close()
            restarted=Terminal(binary,state,project,width=a.width,height=a.height)
            try:restarted.wait('Alt');assert allocation()['output_tokens']==600 and preferences()['workflow']=='host';restarted.close()
            finally:
                if restarted.proc.poll() is None:restarted.proc.terminate();restarted.proc.wait(timeout=10)
            print('PASS: labelled output/sampling/default controls, host workflow, active skill, reviewed-instruction gate, effort/shared/custom budgets, cancel without starting work, saved settings after restart and terminal restoration.')
        finally:
            if term.proc.poll() is None:term.proc.terminate();term.proc.wait(timeout=10)

if __name__=='__main__':main()
