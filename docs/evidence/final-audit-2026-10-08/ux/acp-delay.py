#!/usr/bin/env python3
"""Deterministic ACP peer. No model, network, or shell tools are used."""
import json
import os
import sys
import time

pending = None
waiting = None


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
        time.sleep(2)
        reply(identifier, {"protocolVersion": 1, "agentCapabilities": {"loadSession": True},
                           "agentInfo": {"name": "fixture", "version": "1"}})
    elif method == "session/new":
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
        if prompt == "wait":
            waiting = identifier
            text("waiting")
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
        reply(waiting, {"stopReason": "cancelled"})
        waiting = None
    elif identifier == "permission-1":
        allowed = request["result"]["outcome"].get("optionId") == "yes"
        update("tool_call_update", toolCallId="tool-1", status="completed",
               content=[{"type": "content", "content": {"type": "text", "text": "fixture evidence"}}])
        text("allowed\n" if allowed else "denied\n")
        text("final chunk\n")
        reply(pending, {"stopReason": "end_turn"})
