#!/usr/bin/env python3
"""A beginner's complete repair/check/undo journey via real terminal input."""
import json, os, subprocess, tempfile, time, tomllib
from pathlib import Path
from terminal_harness import Terminal

repo = Path(__file__).resolve().parents[1]
binary = Path(os.environ.get('ALT_TEST_BINARY', repo/'target/debug/alt')).resolve()
screenshots = os.environ.get('ALT_SCREENSHOT_DIR')
def capture(terminal, name, width, height):
    if screenshots:
        from terminal_capture import save_screen
        # A matching label can arrive before the rest of its terminal frame.
        deadline = time.monotonic() + .3
        while time.monotonic() < deadline:
            terminal.pump(.03)
        save_screen(terminal.screen, Path(screenshots)/f'{name}-{width}x{height}')

for width, height in [(120, 40), (80, 24), (60, 18)]:
    with tempfile.TemporaryDirectory(prefix='alt-practice-tui-') as d:
        root = Path(d); state = root/'state'; state.mkdir()
        original = root/'original'; original.mkdir(); (original/'keep.txt').write_text('unchanged')
        (state/'preferences.toml').write_text(f'project={json.dumps(str(original))}\naccess_policy="trusted"\n')
        t = Terminal(binary, state, original, width, height)
        try:
            t.wait('Connect your first model')
            capture(t, 'home', width, height)
            t.send(b'\x1b[B\r'); t.wait('Your first repair')
            capture(t, 'practice', width, height)
            project = Path(tomllib.loads((state/'preferences.toml').read_text())['project'])
            assert project != original and (original/'keep.txt').read_text() == 'unchanged'
            base = [str(binary), '--data-dir', str(state)]
            def verify():
                r = subprocess.run(base+['task','verify'], cwd=project, capture_output=True, text=True)
                return json.loads(r.stdout)
            t.send(b'\r'); t.wait('Structured report contains 3 failing tests'); assert not verify()['complete']
            t.send(b'\x1b9'); t.wait('Project files'); t.send(b'/'); t.wait('Filter project files')
            t.paste('greeting.py'); t.send(b'\r'); t.send(b'e'); t.wait('Edit greeting.py')
            # Replace through the editor; Alt still records a real checkpoint.
            t.send(b'\x15')
            t.paste("def greet(name):\n    return 'Hello, ' + (name.strip() or 'friend') + '!'\n")
            t.send(b'\x13'); t.wait('Save greeting.py?'); t.send(b'\t\r'); t.wait('File saved')
            assert "name.strip()" in (project/'greeting.py').read_text()
            t.send(b'\x1b8'); t.wait('What happened'); t.send(b'r'); t.wait('Run checks again'); t.send(b'\r')
            t.wait('Independent assertion passed'); assert verify()['behavioral_acceptance']
            capture(t, 'verified', width, height)
            t.close(); t = Terminal(binary, state, original, width, height)
            t.wait('Connect your first model'); t.send(b'\x1b8'); t.wait('What happened')
            t.send(b'U'); t.wait("Undo this task's file edits?"); t.send(b'\t\r'); t.wait('Tracked file edits restored')
            t.send(b'r'); t.wait('Run checks again'); t.send(b'\r'); t.wait('Structured report contains 3 failing tests')
            assert not verify()['complete']
            assert (project/'greeting.py').read_text() == "def greet(name):\n    return 'Hello, ' + name + '!'\n"
            print(f'PASS: practice creation, failing seed, edit, four-case independent verification, restart and undo at {width}x{height}', flush=True)
        finally:
            t.close()

with tempfile.TemporaryDirectory(prefix='alt-practice-dependency-') as d:
    root = Path(d); state = root/'state'
    base = [str(binary), '--data-dir', str(state), '--access', 'trusted']
    lesson = json.loads(subprocess.check_output(base+['practice'], text=True))
    project = Path(lesson['project'])
    (state/'preferences.toml').write_text(f'project={json.dumps(str(project))}\naccess_policy="trusted"\n')
    original_path = os.environ['PATH']
    try:
        os.environ['PATH'] = str(root/'unavailable-tools')
        t = Terminal(binary, state, project, 80, 24)
    finally:
        os.environ['PATH'] = original_path
    try:
        t.wait('Connect your first model'); t.send(b'\x1b8'); t.wait('What happened')
        t.send(b'r'); t.wait('Run checks again'); t.send(b'\r'); t.wait('Cannot start python3')
        t.send(b'e'); t.wait('Check evidence'); t.wait('Cannot start python3')
        evidence = json.loads(subprocess.check_output(base+['task','export'], cwd=project, text=True))
        assert 'Install its tools/dependencies' in evidence['checks'][-1]['error']
    finally:
        t.close()
    # Restart with the interpreter available: the same project and check recover.
    t = Terminal(binary, state, project, 80, 24)
    try:
        t.wait('Connect your first model'); t.send(b'\x1b8'); t.wait('What happened')
        t.send(b'r'); t.wait('Run checks again'); t.send(b'\r')
        t.wait('Structured report contains 3 failing tests')
        evidence = json.loads(subprocess.check_output(base+['task','export'], cwd=project, text=True))
        assert evidence['checks'][-1]['tests_run'] == 4
        print('PASS: missing Python is explained in the TUI; restart with the interpreter recovers the same practice check', flush=True)
    finally:
        t.close()
