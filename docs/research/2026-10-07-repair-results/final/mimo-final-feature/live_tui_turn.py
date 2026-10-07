"""A real PTY turn for opt-in live acceptance. No scripted model responses.

The caller must explicitly authorize tools and select an exact uncensored model.
Raw terminal bytes and persisted events survive failures and timeouts.
"""
import fcntl
import json
import os
import pty
import select
import signal
import sqlite3
import struct
import subprocess
import termios
import time
import tomllib

import pyte


def run_turn(base, project, state, prompt, timeout, output, measure, width=80, height=24, allow_tools=False):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
    original = termios.tcgetattr(slave)
    screen = pyte.Screen(width, height)
    stream = pyte.ByteStream(screen)
    begin = time.monotonic()
    proc = None
    events = []
    sent_connect = sent_prompt = False
    approved = set()
    peak = 0
    error = None
    timed_out = False
    restored = False
    observed_screen = ''
    quit_signal_fallback = False
    response_ready_visible = False
    result = {'interface': 'tui', 'width': width, 'height': height}
    selected_access = tomllib.loads((state / 'preferences.toml').read_text()).get('access_policy', 'guided')
    command = list(base)
    if '--access' in command:
        index = command.index('--access')
        assert command[index + 1] == selected_access, 'TUI access must be explicitly saved in preferences; a headless override cannot select it'
        del command[index:index + 2]
    result['access_policy'] = selected_access
    database = state / 'sessions.db'
    checkpoint = 0
    if database.exists():
        with sqlite3.connect('file:' + str(database) + '?mode=ro', uri=True) as db:
            checkpoint = db.execute('SELECT COALESCE(MAX(seq),0) FROM events').fetchone()[0]
    try:
        with (output / 'terminal.raw').open('wb') as raw, (output / 'turn.stderr').open('w') as stderr:
            proc = subprocess.Popen(command + ['tui'], cwd=project, stdin=slave, stdout=slave,
                                    stderr=stderr, start_new_session=True,
                                    env={**os.environ, 'TERM': 'xterm-256color', 'COLORTERM': 'truecolor'})
            deadline = begin + timeout
            while time.monotonic() < deadline:
                if select.select([master], [], [], .1)[0]:
                    try:
                        data = os.read(master, 65536)
                    except OSError:
                        break
                    raw.write(data)
                    stream.feed(data)
                peak = max(peak, measure(proc.pid))
                display = '\n'.join(screen.display)
                if not sent_connect and 'Alt' in display:
                    os.write(master, b'\x0e')  # Ctrl+N: the labelled new-conversation action.
                    sent_connect = True
                if sent_connect and not sent_prompt and ('New conversation' in display or 'Ready. Describe' in display):
                    os.write(master, b'\x1b[200~' + prompt.encode() + b'\x1b[201~\r')
                    sent_prompt = True
                if database.exists():
                    with sqlite3.connect('file:' + str(database) + '?mode=ro', uri=True) as db:
                        events = [json.loads(row[0]) for row in db.execute('SELECT payload FROM events WHERE seq>? ORDER BY seq', [checkpoint])]
                for event in events:
                    if event.get('type') != 'permission_request':
                        continue
                    key = json.dumps(event['id'], sort_keys=True)
                    if key not in approved and 'Review this action' in display:
                        os.write(master, b'\x19' if allow_tools else b'\x0e')  # Ctrl+Y/Ctrl+N: the actual allow/deny choice.
                        approved.add(key)
                if sent_prompt and any(e.get('type') in ('turn_end', 'error') for e in events):
                    break
                if proc.poll() is not None:
                    error = 'TUI exited before the model turn ended'
                    break
            else:
                timed_out = True
                error = 'Live TUI deadline exhausted'
            if any(e.get('type') == 'turn_end' for e in events):
                settle = time.monotonic() + 2
                while time.monotonic() < settle:
                    if select.select([master], [], [], .05)[0]:
                        data = os.read(master, 65536)
                        raw.write(data)
                        stream.feed(data)
                    if 'Response ready' in '\n'.join(screen.display):
                        response_ready_visible = True
                        break
            observed_screen = '\n'.join(screen.display) + '\n'
            if proc.poll() is None:
                if timed_out or not sent_prompt:
                    proc.send_signal(signal.SIGINT)
                else:
                    os.write(master, b'\x11')  # Ctrl+Q once the turn is no longer busy.
                try:
                    proc.wait(timeout=12)
                except subprocess.TimeoutExpired:
                    quit_signal_fallback = True
                    proc.send_signal(signal.SIGINT)
                    try:
                        proc.wait(timeout=8)
                    except subprocess.TimeoutExpired:
                        os.killpg(proc.pid, signal.SIGKILL)
                        proc.wait()
                        error = error or 'TUI required forced termination'
                while select.select([master], [], [], .05)[0]:
                    try:
                        data = os.read(master, 65536)
                    except OSError:
                        break
                    if not data:
                        break
                    raw.write(data)
                    stream.feed(data)
            restored = termios.tcgetattr(slave) == original
            if database.exists():
                with sqlite3.connect('file:' + str(database) + '?mode=ro', uri=True) as db:
                    events = [json.loads(row[0]) for row in db.execute('SELECT payload FROM events WHERE seq>? ORDER BY seq', [checkpoint])]
    except Exception as failure:
        error = repr(failure)
    finally:
        if proc and proc.poll() is None:
            proc.send_signal(signal.SIGINT)
            try:
                proc.wait(timeout=12)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait()
        (output / 'turn.jsonl').write_text(''.join(json.dumps(e, ensure_ascii=False) + '\n' for e in events))
        (output / 'terminal.txt').write_text(observed_screen or '\n'.join(screen.display) + '\n')
        (output / 'terminal-after-exit.txt').write_text('\n'.join(screen.display) + '\n')
        result.update(sent_prompt=sent_prompt, permissions_approved=len(approved) if allow_tools else 0,
                      permissions_denied=0 if allow_tools else len(approved),
                      terminal_restored=restored, error=error, timed_out=timed_out,
                      response_ready_visible=response_ready_visible, quit_signal_fallback=quit_signal_fallback,
                      process_exit=proc.returncode if proc else None,
                      sampled_alt_tree_peak_rss_bytes=peak,
                      wall_seconds=round(time.monotonic() - begin, 2))
        (output / 'tui-turn.json').write_text(json.dumps(result, indent=2) + '\n')
        os.close(master)
        os.close(slave)
    return result
