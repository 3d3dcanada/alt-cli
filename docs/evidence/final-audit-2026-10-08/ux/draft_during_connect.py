import sys,json,tempfile,subprocess,time,os,sqlite3
from pathlib import Path
sys.path.insert(0,'/workspace/alt-cli/scripts')
from terminal_harness import Terminal
out=Path('/workspace/.alt-audit-ux');binary=Path('/workspace/alt-cli/target/debug/alt')
fixture=out/'acp-delay.py';fixture.write_text(Path('/workspace/alt-cli/tests/fixtures/acp_engine.py').read_text().replace('import sys','import sys\nimport time').replace('if method == "initialize":','if method == "initialize":\n        time.sleep(2)'));fixture.chmod(0o755)
with tempfile.TemporaryDirectory(prefix='alt-connect-draft-audit-') as d:
 root=Path(d);state=root/'state';project=root/'project';project.mkdir()
 subprocess.run([str(binary),'--data-dir',str(state),'init','--model','fixture','--endpoint','http://127.0.0.1:1/v1'],check=True,capture_output=True)
 (state/'preferences.toml').write_text('project='+json.dumps(str(project))+'\nengine_path='+json.dumps(str(fixture))+'\n')
 t=Terminal(binary,state,project)
 try:
  t.wait('Start a new conversation');t.send(b'\x1b2');t.wait('Describe what you want');t.paste('ORIGINAL REQUEST');t.send(b'\r');t.wait('Connecting');t.paste(' PLUS IMPORTANT REQUIREMENT');t.wait('PLUS IMPORTANT REQUIREMENT')
  end=time.monotonic()+.3
  while time.monotonic()<end:t.pump()
  (out/'draft-during-connection.txt').write_text('\n'.join(t.screen.display))
  t.wait('Response ready');end=time.monotonic()+.3
  while time.monotonic()<end:t.pump()
  (out/'draft-after-connection.txt').write_text('\n'.join(t.screen.display))
  with sqlite3.connect(state/'sessions.db') as db: events=[json.loads(row[0]) for row in db.execute('SELECT payload FROM events ORDER BY seq')]
  result={'new_requirement_visible_after_connection':'PLUS IMPORTANT REQUIREMENT' in '\n'.join(t.screen.display),'saved_user_messages':[e for e in events if e.get('type')=='user']}
  (out/'draft-during-connect-result.json').write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2))
 finally:t.close()
