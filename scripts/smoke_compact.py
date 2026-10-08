#!/usr/bin/env python3
"""Real Goose + weight-free provider: compact reads, native line edits and checks.

These are deterministic protocol fixtures, not evidence of model capability.
"""
import argparse
import json
import re
import subprocess
import tempfile
import threading
from http.server import ThreadingHTTPServer
from pathlib import Path
from smoke_goose import Provider as BaseProvider

MODEL = 'alt-compact-protocol-fixture'
REQUESTS = []
SOURCE = None
DRIFT = False
BAD_PATCH = False
BROKEN_SYNTAX = False
HTTP_FAILURE = False
FOLLOWUP_GOAL = None
FOLLOWUP_TEXT = 'CONTINUE_SAVED_TASK_FIXTURE: Continue the task.'
FOLLOWUP_EXPECTED = []
EDITOR = 'edit_text'
CREATE_BODY = "VALUE = '☃\\n'\n"


class Provider(BaseProvider):
    def do_POST(self):
        global DRIFT, BAD_PATCH, BROKEN_SYNTAX, HTTP_FAILURE
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        if self.path == '/api/show':
            self.send_json({'capabilities': ['completion', 'tools'], 'model_info': {'llama.context_length': 8192}})
            return
        assert self.path == '/v1/chat/completions'
        assert body['model'] == MODEL
        REQUESTS.append(body)
        if HTTP_FAILURE:
            payload = json.dumps({'error': {'message': 'FIXTURE_BAD_REQUEST', 'type': 'invalid_request_error', 'code': 'bad_request'}}).encode()
            self.send_response(400)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        tools = {t['function']['name'].split('__')[-1]: t['function']['name'] for t in body['tools']}
        assert EDITOR in tools and 'edit' not in tools
        assert all(name in tools for name in ['terminal', 'skill', 'evidence', 'run_check'])
        results = [m for m in body['messages'] if m['role'] == 'tool']
        tool = None
        arguments = None
        if FOLLOWUP_GOAL:
            content = '\n'.join(str(m.get('content', '')) for m in body['messages'] if m['role'] == 'user')
            assert content.count(FOLLOWUP_GOAL) == 1, 'Complete original task scope must appear once, without losing its final requirement'
            if FOLLOWUP_TEXT in content:
                assert 'Original task request (context; the latest user message takes precedence)' in content
            else:
                assert 'Original task request (context;' not in content, 'First request was needlessly duplicated'
            current = [int(value) for value in re.findall(r'KEEP_INTERVENING_(\d{2})', content)]
            if current:
                for requirement in FOLLOWUP_EXPECTED[:max(current)]:
                    assert requirement in content, 'An exact intervening user requirement was lost from the actual provider request'
        elif any(m['role'] == 'user' and 'CREATE_DELETE_FIXTURE' in str(m.get('content')) for m in body['messages']):
            if not results:
                tool = 'create_file' if EDITOR == 'edit_text' else 'create_lines'
                arguments = {'path': 'addon.py', **({'new_text': CREATE_BODY} if EDITOR == 'edit_text' else {'lines': [CREATE_BODY.rstrip('\n')]})}
            elif 'Current file addon.py' in str(results[-1]['content']):
                handle = re.search(r'handle (h\d+-[a-f0-9]+):', str(results[-1]['content']))[1]
                tool, arguments = 'delete_file', {'path': 'addon.py', 'handle': handle}
            elif 'File deleted.' in str(results[-1]['content']):
                tool, arguments = 'run_check', {'name': 'behavior'}
        elif not results:
            content = '\n'.join(str(m.get('content', '')) for m in body['messages'] if m['role'] == 'user')
            handle = re.search(r'Current file repair.py,.*?handle (h\d+-[a-f0-9]+):', content)[1]
            assert 'def scaled(x):\n return x * 2\n' in content
            if DRIFT:
                SOURCE.write_text('def scaled(x):\n return x * 4\n')
                DRIFT = False
            tool, arguments = 'edit_lines', {'path': 'repair.py', 'handle': handle, 'lines': ['def scaled(x):', ' return x * 3']}
            if BAD_PATCH:
                arguments['lines'][1] = ' return x * 5'
                BAD_PATCH = False
            if BROKEN_SYNTAX:
                arguments['lines'] = [' return x * 3']
                arguments['intentional'] = True  # Deliberate incomplete source tests explicit rewrites and recovery.
                BROKEN_SYNTAX = False
        else:
            last = str(results[-1]['content'])
            if 'Stale or mismatched handle' in last:
                tool, arguments = 'read', {'path': 'repair.py'}
            elif 'Exit code: 1' in last:
                updated = next(str(m['content']) for m in reversed(results) if 'Applied checkpoint' in str(m['content']))
                handle = re.search(r'handle (h\d+-[a-f0-9]+):', updated)[1]
                if 'Previous extracted definitions missing from current source: ["scaled"]' in updated:
                    lines = ['def scaled(x):', ' return x * 3']
                else:
                    assert 'return x * 5' in updated
                    lines = [' return x * 3']
                tool, arguments = 'edit_lines', {'path': 'repair.py', 'handle': handle, 'lines': lines}
            elif 'checkpoint' in last:
                tool, arguments = 'run_check', {'name': 'behavior'}
            elif 'Current file repair.py' in last:
                handle = re.search(r'handle (h\d+-[a-f0-9]+):', last)[1]
                tool, arguments = 'edit_lines', {'path': 'repair.py', 'handle': handle, 'lines': ['def scaled(x):', ' return x * 3']}
        if tool:
            if tool == 'edit_lines':
                tool = EDITOR
                if EDITOR == 'edit_text':
                    arguments['new_text'] = '\n'.join(arguments.pop('lines'))
            message = {'role': 'assistant', 'content': None, 'tool_calls': [{'index': 0, 'id': f'fixture-{len(REQUESTS)}', 'type': 'function', 'function': {'name': tools[tool], 'arguments': json.dumps(arguments)}}]}
            finish = 'tool_calls'
        else:
            message = {'role': 'assistant', 'content': 'Fixture observed complete task scope; source intentionally unchanged.' if FOLLOWUP_GOAL else 'Actual tool result: ' + str(results[-1]['content'])}
            finish = 'stop'
        self.send_response(200)
        self.send_header('Content-Type', 'text/event-stream')
        self.end_headers()
        chunk = {'id': 'fixture-response', 'model': MODEL, 'choices': [{'index': 0, 'delta': message, 'finish_reason': finish}], 'usage': {'prompt_tokens': 120, 'completion_tokens': 40, 'total_tokens': 160}}
        self.wfile.write(('data: ' + json.dumps(chunk) + '\n\ndata: [DONE]\n\n').encode())
        self.wfile.flush()


def main():
    global SOURCE, DRIFT, BAD_PATCH, BROKEN_SYNTAX, EDITOR, HTTP_FAILURE, FOLLOWUP_GOAL, FOLLOWUP_EXPECTED
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--alt', type=Path, default=Path('target/debug/alt'))
    parser.add_argument('--goose', type=Path, required=True)
    parser.add_argument('--provider', choices=['openai', 'ollama'], default='openai')
    parser.add_argument('--tool-profile', choices=['compact','compact-lines'], default='compact')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--conversation-turns', type=int, default=2, help='Actual continuation turns, 2..50')
    parser.add_argument('--git-project', action='store_true', help='Use a committed project with a separate dirty user file')
    parser.add_argument('--interface', choices=['cli','tui'], default='cli')
    args = parser.parse_args()
    assert 2 <= args.conversation_turns <= 50
    EDITOR = 'edit_lines' if args.tool_profile == 'compact-lines' else 'edit_text'
    server = ThreadingHTTPServer(('127.0.0.1', 0), Provider)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    rows = []
    try:
        with tempfile.TemporaryDirectory(prefix='alt-compact-smoke-') as directory:
            root = Path(directory)
            project = root / 'project'
            project.mkdir()
            SOURCE = project / 'repair.py'
            user_file=project/'keep.py'
            if args.git_project:
                SOURCE.write_text('def scaled(x):\n return x * 2\n')
                user_file.write_text('# Existing user module\nVALUE = 1\n')
                subprocess.run(['git','init','-q',str(project)],check=True)
                subprocess.run(['git','add','repair.py','keep.py'],cwd=project,check=True)
                subprocess.run(['git','-c','user.name=Alt protocol fixture','-c','user.email=fixture@example.invalid','-c','commit.gpgsign=false','commit','-qm','Seed project'],cwd=project,check=True)
                user_file.write_text('# Existing user module\nVALUE = 2  # Uncommitted user change ☃\n')
                user_bytes=user_file.read_bytes()
            state = root / 'state'
            base = [str(args.alt.resolve()), '--data-dir', str(state), '--engine', str(args.goose.resolve()), '--access', 'trusted']
            endpoint = f'http://127.0.0.1:{server.server_port}' + ('/v1' if args.provider == 'openai' else '')

            def run(*arguments):
                if arguments[0] == 'run' and args.interface == 'tui':
                    from live_tui_turn import run_turn
                    destination = args.output.parent / f'{args.output.stem}-tui-{len(rows)}' if args.output else root / f'tui-{len(rows)}'
                    destination.mkdir(parents=True)
                    receipt = run_turn(base, project, state, arguments[1], 60, destination, lambda _: 0, allow_tools='--allow-tools' in arguments)
                    assert receipt['error'] is None and receipt['terminal_restored'], receipt
                    assert receipt['response_ready_visible'] and not receipt['quit_signal_fallback'], receipt
                    return (destination / 'turn.jsonl').read_text()
                result = subprocess.run(base + list(arguments), cwd=project, text=True, capture_output=True, timeout=60)
                assert result.returncode == 0, (result.stdout[-4000:], result.stderr)
                return result.stdout

            run('init', '--model', MODEL, '--endpoint', endpoint, '--provider', args.provider, '--uncensored')
            run('tools', args.tool_profile)
            run('workflow', 'host')
            if args.interface == 'tui':
                preferences = state / 'preferences.toml'
                value, count = re.subn(r'^access_policy = "[^"]+"$', 'access_policy = "trusted"', preferences.read_text(), flags=re.MULTILINE)
                assert count == 1, 'Fixture must explicitly prepare saved TUI access'
                preferences.write_text(value)
            assertion = root / 'assertion.py'
            assertion.write_text('from repair import scaled\nassert scaled(4)==12\nassert scaled(0)==0\nassert scaled(-2)==-6\nprint("BEHAVIOR_OK")\n')
            run('task', 'configure-check', 'behavior', '--', 'python3', '-c', f'exec(open({str(assertion)!r}).read())')
            for scenario in ['denied', 'allowed', 'stale-and-recovered', 'check-failure-and-recovered', 'missing-definition-and-recovered']:
                SOURCE.write_text('def scaled(x):\n return x * 2\n')
                DRIFT = scenario == 'stale-and-recovered'
                BAD_PATCH = scenario == 'check-failure-and-recovered'
                BROKEN_SYNTAX = scenario == 'missing-definition-and-recovered'
                request_offset = len(REQUESTS)
                events = [json.loads(line) for line in run('run', 'Repair scaled(x) in repair.py so it multiplies x by three. Preserve its interface.', '--json', *(['--allow-tools'] if scenario != 'denied' else [])).splitlines()]
                if scenario == 'denied':
                    assert 'return x * 2' in SOURCE.read_text()
                    assert any(e['type']=='permission_decision' and not e['allow'] for e in events)
                else:
                    assert SOURCE.read_text() == 'def scaled(x):\n return x * 3\n'
                    assert any(e['type']=='permission_decision' and e['allow'] for e in events)
                    assert 'BEHAVIOR_OK' in json.dumps(events)
                    if scenario == 'stale-and-recovered':
                        assert 'Stale or mismatched handle' in json.dumps(events)
                        assert any(t['function']['name'].endswith('read') for b in REQUESTS[request_offset:] for m in b['messages'] if m['role']=='assistant' for t in m.get('tool_calls', []))
                    if scenario == 'check-failure-and-recovered':
                        assert 'Exit code: 1' in json.dumps(events)
                        assert not any(t['function']['name'].endswith('read') for b in REQUESTS[request_offset:] for m in b['messages'] if m['role']=='assistant' for t in m.get('tool_calls', []))
                    if scenario == 'missing-definition-and-recovered':
                        assert 'Previous extracted definitions missing' in json.dumps(events)
                        assert 'Exit code: 1' in json.dumps(events)
                rows.append({'scenario':scenario,'source':SOURCE.read_text(),'events':events,'requests':REQUESTS[request_offset:]})
            request_offset = len(REQUESTS)
            events = [json.loads(line) for line in run('run', 'CREATE_DELETE_FIXTURE: create addon.py with the requested literal value, then delete it and run behavior.', '--json', '--allow-tools').splitlines()]
            assert not (project / 'addon.py').exists()
            assert 'BEHAVIOR_OK' in json.dumps(events)
            changes = json.loads(run('task', 'changes'))
            created = next(c for c in changes if c['path'] == 'addon.py' and c['before'] is None)
            deleted = next(c for c in changes if c['path'] == 'addon.py' and c['after'] is None)
            assert created['after'] == CREATE_BODY and deleted['before'] == CREATE_BODY
            run('task', 'undo', deleted['id'])
            assert (project / 'addon.py').read_text() == CREATE_BODY
            run('task', 'undo', created['id'])
            assert not (project / 'addon.py').exists()
            rows.append({'scenario':'create-delete-and-undo','events':events,'changes':[created,deleted],'requests':REQUESTS[request_offset:]})
            request_offset = len(REQUESTS)
            original_source = SOURCE.read_text()
            FOLLOWUP_GOAL = 'Repair scaled(x) in repair.py. ' + 'Preserve its original public interface. ' * 10 + 'FINAL_SCOPE_MARKER ☃.'
            FOLLOWUP_EXPECTED = [f'{FOLLOWUP_TEXT} Preserve KEEP_INTERVENING_{turn:02} ☃.' for turn in range(1, args.conversation_turns)]
            if args.interface == 'tui':
                from live_tui_turn import run_turn
                destination = args.output.parent / f'{args.output.stem}-tui-scope' if args.output else root / 'tui-scope'
                destination.mkdir(parents=True)
                receipt = run_turn(base, project, state, FOLLOWUP_GOAL, 60, destination, lambda _: 0, followup_prompts=FOLLOWUP_EXPECTED)
                assert receipt['error'] is None and receipt['terminal_restored'] and receipt['response_ready_visible'] and not receipt['quit_signal_fallback'], receipt
                assert receipt['turns_completed'] == receipt['prompts_sent'] == receipt['turns_requested'] == args.conversation_turns, receipt
                events = [json.loads(line) for line in (destination / 'turn.jsonl').read_text().splitlines()]
            else:
                first = [json.loads(line) for line in run('run', FOLLOWUP_GOAL, '--json').splitlines()]
                session = next(e['session']['id'] for e in first if e.get('type') == 'session')
                events=first
                for prompt in FOLLOWUP_EXPECTED:
                    events += [json.loads(line) for line in run('run', prompt, '--json', '--resume', session).splitlines()]
                assert sum(e.get('type') == 'turn_end' and e.get('data', {}).get('stopReason') == 'end_turn' for e in events) == args.conversation_turns
            assert len(REQUESTS[request_offset:]) == args.conversation_turns, 'Scope fixture requires one actual model request per turn'
            assert SOURCE.read_text() == original_source, 'Scope-only fixture must not manufacture a repair'
            rows.append({'scenario':'continuation-keeps-original-goal','scope':'Actual continuation context/transport with distinct intervening requirements; source intentionally unchanged; no model weights','turns':args.conversation_turns,'events':events,'requests':REQUESTS[request_offset:],'original_goal':FOLLOWUP_GOAL,'exact_intervening_requests':FOLLOWUP_EXPECTED})
            FOLLOWUP_GOAL = None
            SOURCE.write_text('def scaled(x):\n return x * 2\n')
            run('inference', '--requests', '1')
            request_offset = len(REQUESTS)
            exhausted = subprocess.run(base + ['run', 'Repair scaled(x) in repair.py so it multiplies x by three. Preserve its interface.', '--json', '--allow-tools'], cwd=project, text=True, capture_output=True, timeout=60)
            events = [json.loads(line) for line in exhausted.stdout.splitlines()]
            assert exhausted.returncode != 0, 'Exhausted allowance reported a successful turn'
            assert 'Model call allowance exhausted' in exhausted.stderr, exhausted.stderr
            assert len(REQUESTS[request_offset:]) == 1, 'An unapproved extra model request was sent'
            assert SOURCE.read_text() == 'def scaled(x):\n return x * 3\n', 'Applied source was lost on exhaustion'
            session = next(e['session']['id'] for e in events if e.get('type') == 'session')
            assert 'Applied checkpoint' in run('export', session), 'Checkpoint receipt was lost on exhaustion'
            rows.append({'scenario':'allowance-exhausted','source':SOURCE.read_text(),'events':events,'requests':REQUESTS[request_offset:],'exit':exhausted.returncode,'stderr':exhausted.stderr})
            run('inference', '--reset')
            SOURCE.write_text('def scaled(x):\n return x * 2\n')
            HTTP_FAILURE = True
            request_offset = len(REQUESTS)
            bad_request = subprocess.run(base + ['run', 'Repair scaled(x) in repair.py so it multiplies x by three. Preserve its interface.', '--json', '--allow-tools'], cwd=project, text=True, capture_output=True, timeout=60)
            if args.output:
                args.output.parent.mkdir(parents=True, exist_ok=True)
                args.output.with_suffix('.bad-request.json').write_text(json.dumps({'scope':'Scripted persistent HTTP 400; real Goose/Alt boundary','exit':bad_request.returncode,'stdout':bad_request.stdout,'stderr':bad_request.stderr,'requests':REQUESTS[request_offset:]},indent=2)+'\n')
            assert bad_request.returncode != 0, 'Provider rejection reported a successful turn'
            assert 'Model request failed' in bad_request.stderr, bad_request.stderr
            assert len(REQUESTS[request_offset:]) >= 1
            assert SOURCE.read_text() == 'def scaled(x):\n return x * 2\n'
            rows.append({'scenario':'provider-bad-request','source':SOURCE.read_text(),'events':[json.loads(line) for line in bad_request.stdout.splitlines()],'requests':REQUESTS[request_offset:],'exit':bad_request.returncode,'stderr':bad_request.stderr})
            export = json.loads(run('task', 'export'))
            assert any(c['status']=='rejected' for c in export['changes'])
            assert sum(c['status']=='applied' for c in export['changes']) == 7
            if args.git_project:
                assert user_file.read_bytes()==user_bytes, 'Uncommitted user changes were overwritten'
                assert subprocess.check_output(['git','status','--porcelain','--','keep.py'],cwd=project,text=True).strip()=='M keep.py'
        if args.output:
            for row in rows:
                row['interface'] = 'cli' if row['scenario'] in ['allowance-exhausted','provider-bad-request'] else args.interface
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps({'scope':'Real Goose and MCP with scripted weight-free responses; protocol only','provider':args.provider,'tool_profile':args.tool_profile,'interface':args.interface,'conversation_turns':args.conversation_turns,'git_project':args.git_project,'rows':rows},indent=2)+'\n')
        print(f'PASS: Goose/{args.provider}: {args.tool_profile} initial read, native {EDITOR} edits, denial, checkpoints, checks, stale-handle and failed-check recovery, exhausted-allowance failure with saved source/history.')
    finally:
        server.shutdown()
        server.server_close()


if __name__ == '__main__':
    main()
