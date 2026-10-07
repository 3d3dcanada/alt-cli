#!/usr/bin/env python3
"""Real Goose + deterministic HTTP provider, NOT a live-model quality test.

Runs the actual Alt-owned terminal tool in explicit Full access mode in a temporary workspace, checks denial,
explicit approval, evidence persistence, session reload, and model inventory.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import signal
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

REQUESTS = []
MODEL = "alt-protocol-fixture"
STALL_STARTED = threading.Event()
STALL_RELEASE = threading.Event()


class Provider(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_GET(self):
        if self.path == "/v1/models":
            value = {"object": "list", "data": [{"id": MODEL, "object": "model"}]}
        elif self.path == "/api/tags":
            value = {"models": [{"name": MODEL, "model": MODEL}]}
        elif self.path == "/props":
            value = {"default_generation_settings": {"n_ctx": 8192}}
        else:
            self.send_error(404)
            return
        self.send_json(value)

    def send_json(self, value):
        body = json.dumps(value).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if self.path == "/api/show":
            self.send_json({"capabilities": ["completion", "tools"], "model_info": {"general.architecture": "llama", "llama.context_length": 8192}})
            return
        REQUESTS.append((self.path, request))
        if self.path != "/v1/chat/completions":
            self.send_error(404)
            return
        messages = request["messages"]
        if any(m["role"] == "user" and "STALL_FIXTURE" in str(m.get("content")) for m in messages):
            STALL_STARTED.set()
            STALL_RELEASE.wait(timeout=15)
            return
        tool_results = [m for m in messages if m["role"] == "tool"]
        if any(m["role"] == "user" and "Report the previous evidence" in str(m.get("content")) for m in messages):
            memory = json.dumps(messages)
            message = {"role": "assistant", "content": "Restored memory contains ALT_TOOL_OK" if "ALT_TOOL_OK" in memory else "Memory missing"}
            finish = "stop"
        elif not tool_results and any(m["role"]=="user" and "EXTERNAL_FIXTURE" in str(m.get("content")) for m in messages):
            tool=next(t["function"]["name"] for t in request["tools"] if t["function"]["name"].endswith("counter"))
            message={"role":"assistant","content":None,"tool_calls":[{"id":"external-fixture-call","type":"function","function":{"name":tool,"arguments":"{}"}}]}
            finish="tool_calls"
        elif tool_results:
            # Echo evidence from the actual tool result, never invent success.
            content = "Observed tool result: " + json.dumps(tool_results[-1].get("content"))
            message = {"role": "assistant", "content": content}
            finish = "stop"
        else:
            shells = [t["function"] for t in request.get("tools", []) if t["function"]["name"].endswith("terminal")]
            if not shells:
                message = {"role": "assistant", "content": "No Alt terminal tool was exposed"}
                finish = "stop"
            else:
                command = 'python3 -c \'from pathlib import Path; Path("fixture-evidence.txt").write_text("ALT_TOOL_OK"); print("ALT_TOOL_OK")\''
                message = {"role": "assistant", "content": None, "tool_calls": [{"id": "fixture-call", "type": "function",
                           "function": {"name": shells[0]["name"], "arguments": json.dumps({"command": command})}}]}
                finish = "tool_calls"
        response = {"id": "fixture-response", "object": "chat.completion", "created": 1, "model": MODEL,
                    "choices": [{"index": 0, "message": message, "finish_reason": finish}],
                    "usage": {"prompt_tokens": 100, "completion_tokens": 25, "total_tokens": 125}}
        if request.get("stream"):
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            delta = dict(message)
            if "tool_calls" in delta:
                delta["tool_calls"][0]["index"] = 0
            chunk = {"id": "fixture-response", "object": "chat.completion.chunk", "created": 1, "model": MODEL,
                     "choices": [{"index": 0, "delta": delta, "finish_reason": None}]}
            final = {**chunk, "choices": [{"index": 0, "delta": {}, "finish_reason": finish}], "usage": response["usage"]}
            for item in [json.dumps(chunk), json.dumps(final), "[DONE]"]:
                self.wfile.write(("data: " + item + "\n\n").encode())
            self.wfile.flush()
        else:
            self.send_json(response)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--alt", default="target/debug/alt")
    parser.add_argument("--goose", required=True)
    parser.add_argument("--provider", choices=["openai", "ollama"], default="openai")
    args = parser.parse_args()
    alt, goose = str(Path(args.alt).resolve()), str(Path(args.goose).resolve())
    server = ThreadingHTTPServer(("127.0.0.1", 0), Provider)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with tempfile.TemporaryDirectory(prefix="alt-goose-smoke-") as temp:
            root = Path(temp)
            workspace = root / "workspace"
            workspace.mkdir()
            endpoint = f"http://127.0.0.1:{server.server_port}" + ("/v1" if args.provider == "openai" else "")

            def run(*arguments):
                result = subprocess.run([alt, "--data-dir", str(root / "state"), "--engine", goose, "--access", "trusted", *arguments],
                                        cwd=workspace, capture_output=True, text=True, timeout=90)
                if result.returncode:
                    raise AssertionError(f"alt {arguments} failed\n{result.stderr}\n{result.stdout[-10000:]}")
                return result.stdout

            run("init", "--model", MODEL, "--endpoint", endpoint, "--provider", args.provider, "--context", "8192")
            print(run("doctor").strip())
            denied = [json.loads(line) for line in run("run", "Run fixture check", "--json").splitlines()]
            assert not (workspace / "fixture-evidence.txt").exists(), "Unapproved shell tool executed"
            assert any(e["type"] == "permission_decision" and not e["allow"] for e in denied), json.dumps(denied)[-10000:]
            allowed = [json.loads(line) for line in run("run", "Run fixture check", "--allow-tools", "--json").splitlines()]
            assert (workspace / "fixture-evidence.txt").read_text() == "ALT_TOOL_OK"
            assert any(e["type"] == "permission_decision" and e["allow"] for e in allowed)
            assert any(e["type"] == "update" and "ALT_TOOL_OK" in json.dumps(e) for e in allowed)
            session = allowed[0]["session"]["id"]
            resumed = run("run", "Report the previous evidence", "--resume", session)
            assert "ALT_TOOL_OK" in resumed, repr(resumed)
            exported = run("export", session)
            assert "permission_decision" in exported and "ALT_TOOL_OK" in exported
            connection=root/'mcp.json'
            connection.write_text(json.dumps({"name":"fixture_ext","transport":{"type":"stdio","command":"python3","args":[str(Path(__file__).resolve().parents[1]/'tests/fixtures/mcp_server.py')]},"enabled":False,"selected_tools":[]}))
            run('extensions','add',str(connection));run('extensions','check','fixture_ext');run('extensions','select','fixture_ext','counter')
            external=[json.loads(line) for line in run('run','EXTERNAL_FIXTURE call the counter tool','--allow-tools','--json').splitlines()]
            assert any(e['type']=='permission_decision' and e['allow'] for e in external),external
            assert any(e['type']=='update' and 'Observed tool result:' in json.dumps(e) and '1' in json.dumps(e) for e in external),external
            run('extensions','disable','fixture_ext')
            (workspace/'fixture-evidence.txt').unlink()
            plain=subprocess.run([alt,'--data-dir',str(root/'state'),'--engine',goose,'--access','trusted','plain'],input='Run fixture check\nn\nRun fixture check\ny\n/quit\n',cwd=workspace,capture_output=True,text=True,timeout=90)
            assert plain.returncode==0,(plain.stdout,plain.stderr)
            assert 'Allow once? [y/N]' in plain.stdout and 'ALT_TOOL_OK' in plain.stdout,(plain.stdout,plain.stderr)
            assert (workspace/'fixture-evidence.txt').read_text()=='ALT_TOOL_OK'
            cancellation = subprocess.Popen([alt, "--data-dir", str(root / "state"), "--engine", goose,
                                              "run", "STALL_FIXTURE", "--json"], cwd=workspace,
                                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                assert STALL_STARTED.wait(timeout=15), "Engine never called the stalled fixture"
                processes = subprocess.check_output(["ps", "-eo", "pid=,ppid="], text=True)
                engine_pids = [int(pid) for pid, parent in (line.split() for line in processes.splitlines())
                               if int(parent) == cancellation.pid]
                assert engine_pids, "No child engine to verify cleanup"
                cancellation.send_signal(signal.SIGINT)
                out, err = cancellation.communicate(timeout=10)
                # Goose 1.53.0 does not interrupt a provider stalled before HTTP
                # headers. Assert Alt's documented five-second kill fallback.
                assert cancellation.returncode != 0 and "Engine did not stop within 5 seconds" in err, (out[-2000:], err)
                remaining = subprocess.check_output(["ps", "-eo", "pid="], text=True).split()
                assert all(str(pid) not in remaining for pid in engine_pids), "Owned engine leaked after cancellation"
            finally:
                STALL_RELEASE.set()
                if cancellation.poll() is None:
                    cancellation.send_signal(signal.SIGINT)
                    try:
                        cancellation.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        cancellation.kill()
                        cancellation.wait()
            assert all(path == "/v1/chat/completions" for path, _ in REQUESTS)
            assert all(body["model"] == MODEL for _, body in REQUESTS)
            assert all(body.get("max_tokens") == 2048 for _, body in REQUESTS), "Actual server output limit missing"
            assert all("options" not in body for _, body in REQUESTS), "Native-only fields leaked into compatible chat requests"
            assert any("native tool calls" in json.dumps(body["messages"]) for _, body in REQUESTS), "Operator instructions missing"
            tools = sorted({tool["function"]["name"] for _, body in REQUESTS for tool in body.get("tools", [])})
            assert sorted(name.split("__")[-1] for name in tools) == ["counter", "edit", "evidence", "list", "read", "remember", "run_check", "search", "skill", "terminal"], tools
            print(f"Tool inventory ({len(tools)}): {', '.join(tools)}")
            print(f"PASS: real Goose / {args.provider}: inventory, ACP, denial, approval, shell evidence, export, resume, stalled-provider kill fallback, instructions ({len(REQUESTS)} fixture inference requests).")
    finally:
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    main()
