#!/usr/bin/env python3
"""Configure and run a pinned behavioral assertion entirely through labelled TUI choices."""
import json,os,subprocess,tempfile,tomllib
from pathlib import Path
from terminal_harness import Terminal
repo=Path(__file__).resolve().parents[1];binary=Path(os.environ.get('ALT_TEST_BINARY',repo/'target/debug/alt')).resolve()
with tempfile.TemporaryDirectory(prefix='alt-contract-tui-') as d:
 root=Path(d);project=root/'project';project.mkdir();state=root/'state';state.mkdir();(project/'answer.txt').write_text('42')
 (state/'preferences.toml').write_text('project='+json.dumps(str(project))+'\naccess_policy="trusted"\n')
 assertion=root/'assertion.py';assertion.write_text('import os,json\nfrom pathlib import Path\nassert Path("answer.txt").read_text()=="42"\nPath(os.environ["ALT_CHECK_REPORT"]).write_text(json.dumps({"schema":1,"complete":True,"tests":[{"name":"answer is 42","status":"passed"}]}))\n')
 base=[str(binary),'--data-dir',str(state)]
 for args in [['task','configure-check','answer','--','true'],['task','require','answer-behavior','--check','answer']]:subprocess.run(base+args,cwd=project,check=True,capture_output=True)
 t=Terminal(binary,state,project)
 try:
  t.wait('Connect your first model');t.send(b'\x1b8');t.wait('What happened');t.send(b'c');t.wait('Choose what should prove');t.send(b'\x1b[B\r');t.wait('Which check needs evidence?');t.send(b'\r');t.wait('What does this check establish?');t.send(b'\r');t.wait('Test report format');t.send(b'\r');t.wait('Where will the report');t.send(b'\r');t.wait('Who owns the behavioral assertion?');t.send(b'\x1b[B\r');t.wait('Independent assertion file');t.paste(str(assertion));t.send(b'\r');t.wait('Save this verification contract?');t.send(b'\t\r');t.wait('Contract saved');t.send(b'r');t.wait('Run checks again');t.send(b'\r');t.wait('Independent assertion passed')
  # Independently inspect persisted evidence, not just UI text.
  result=json.loads(subprocess.check_output(base+['task','verify'],cwd=project,text=True));assert result['behavioral_acceptance'] and result['complete'],result
  t.send(b'e');t.wait('Check evidence');t.send(b'\r')
  t.send(b'\x1b6');t.wait('Preferences');t.send(b'\x1b[B'*15+b'\r');t.wait('Which tools fit this task?');t.send(b'\x1b[B'*2+b'\r');t.wait('Tool focus saved')
  t.send(b'\x1b[A'*2+b'\r');t.wait('Managed runtime settings');t.send(b'\x1b[B'*6+b'\r');t.wait("Choose the model's reasoning mode");t.send(b'\x1b[B'*2+b'\r');t.wait('Reasoning choice saved')
  assert tomllib.loads((state/'preferences.toml').read_text())['runtime']['thinking'] is False
  print('PASS: labelled contract wizard, external assertion pinning, structured evidence, behavioral badge, tool-focus and reasoning-mode selection')
 finally:t.close()
