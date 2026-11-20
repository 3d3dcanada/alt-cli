#!/usr/bin/env python3
"""Deterministic ACP peer. No model, network, or shell tools are used."""
import json
import os
import subprocess
import sys
import threading
import urllib.request

pending = None
waiting = None
failing = False
servers = []


def send(value):
    print(json.dumps({"jsonrpc": "2.0", **value}), flush=True)


def reply(identifier, result):
    send({"id": identifier, "result": result})


def update(kind, **fields):
    send({"method": "session/update", "params": {
        "sessionId": "fixture-session", "update": {"sessionUpdate": kind, **fields}}})


def text(value):
    update("agent_message_chunk", content={"type": "text", "text": value})


for line in sys.stdin:
    request = json.loads(line)
    if os.environ.get("ALT_FIXTURE_TRACE"):
        with open(os.environ["ALT_FIXTURE_TRACE"], "a") as trace:
            trace.write(json.dumps(request) + "\n")
    method = request.get("method")
    identifier = request.get("id")
    if method == "initialize":
        reply(identifier, {"protocolVersion": 1, "agentCapabilities": {"loadSession": True},
                           "agentInfo": {"name": "fixture", "version": "1"}})
    elif method == "session/new":
        # Like Goose 1.53: start each listed MCP server and finish its handshake
        # before answering. ALT_FIXTURE_NO_TOOLS models an engine that drops them.
        for server in servers:
            server.kill()
        servers = []
        if not os.environ.get("ALT_FIXTURE_NO_TOOLS"):
            for extension in request["params"].get("_meta", {}).get("enabledExtensions", []):
                if extension.get("type") != "mcp" or extension["server"]["name"] != "alt":
                    continue
                spec = extension["server"]
                server = subprocess.Popen([spec["command"], *spec["args"]], stdin=subprocess.PIPE,
                                          stdout=subprocess.PIPE, text=True)
                server.stdin.write(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                    "protocolVersion": "2024-11-05", "capabilities": {},
                    "clientInfo": {"name": "fixture", "version": "1"}}}) + "\n")
                server.stdin.flush()
                server.stdout.readline()
                servers.append(server)
        reply(identifier, {"sessionId": "fixture-session"})
    elif method == "session/close":
        reply(identifier, {})
    elif method == "session/load":
        # More replay chunks than the bounded event queue: setup must drain it.
        for _ in range(200):
            text("replay")
        reply(identifier, {})
    elif method == "session/prompt":
        prompt = request["params"]["prompt"][-1]["text"]
        if prompt == "crash":
            sys.exit(7)
        if prompt == "bad-json":
            print("not json", flush=True)
            continue
        if prompt == "oversize":
            print("x" * (1024 * 1024 + 1), flush=True)
            continue
        if prompt in ("wait", "wait-error"):
            # wait-error answers the cancel with an error, like an engine whose
            # provider connection was cut.
            waiting = identifier
            failing = prompt == "wait-error"
            text("waiting")
            continue
        if prompt in ("turn-limit", "model-says-limit"):
            # Goose 1.53's exact turn-limit message; only the engine's msg_ id marks it.
            update("agent_message_chunk", content={"type": "text", "text": "I've reached the maximum number of actions I can do without user input. Would you like me to continue?"},
                   _meta={"goose": {"messageId": "msg_fixture" if prompt == "turn-limit" else "chatcmpl-fixture"}})
            reply(identifier, {"stopReason": "end_turn"})
            continue
        if prompt == "stuck":
            # Never answers, not even a cancel (an engine blocked on a provider).
            text("stuck")
            continue
        if prompt == "slow-model":
            # One model call through Alt's relay, like a real engine turn.
            def call(identifier=identifier):
                body = json.dumps({"model": "fixture", "stream": False,
                                   "messages": [{"role": "user", "content": "hi"}]}).encode()
                request = urllib.request.Request(
                    os.environ["OPENAI_BASE_URL"] + "/chat/completions", data=body,
                    headers={"Content-Type": "application/json"})
                with urllib.request.urlopen(request, timeout=120) as response:
                    response.read()
                text("model answered\n")
                reply(identifier, {"stopReason": "end_turn"})
            threading.Thread(target=call, daemon=True).start()
            continue
        if prompt == "tools":
            pending = identifier
            text("Checking…\n")
            update("tool_call", toolCallId="tool-1", title="fixture check", status="pending")
            send({"id": "permission-1", "method": "session/request_permission", "params": {
                "sessionId": "fixture-session", "toolCall": {"toolCallId": "tool-1", "title": "fixture check", "rawInput": {"command": "fixture-only"}},
                "options": [{"optionId": "yes", "name": "Allow", "kind": "allow_once"},
                            {"optionId": "no", "name": "Reject", "kind": "reject_once"}]}})
        else:
            text("fixture response λ\n")
            text("final chunk\n")
            reply(identifier, {"stopReason": "end_turn"})
    elif method == "session/cancel" and waiting is not None:
        if failing:
            send({"id": waiting, "error": {"code": -32603, "message": "Network error: connection closed"}})
        else:
            reply(waiting, {"stopReason": "cancelled"})
        waiting = None
    elif identifier == "permission-1":
        allowed = request["result"]["outcome"].get("optionId") == "yes"
        update("tool_call_update", toolCallId="tool-1", status="completed",
               content=[{"type": "content", "content": {"type": "text", "text": "fixture evidence"}}])
        text("allowed\n" if allowed else "denied\n")
        text("final chunk\n")
        reply(pending, {"stopReason": "end_turn"})
