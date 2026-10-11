#!/usr/bin/env python3
"""A deterministic llama-server stand-in for runtime tests. No model weights.

Records its argv as JSON to $ALT_FAKE_ARGV (when set), serves /health without
authentication like llama.cpp, and requires the bearer key read from
--api-key-file (or --api-key) on every /v1 route and /props.
"""
import json
import os
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

argv = sys.argv[1:]
if argv[:1] in (["--list-devices"], ["--help"]):
    print("--reasoning-budget N\n--api-key-file FNAME\nAvailable devices:")
    if os.environ.get("ALT_FAKE_GPU"):
        print("  CUDA0: NVIDIA GeForce GTX 1070 (8192 MiB, 7900 MiB free)")
    sys.exit(0)


def option(name, default=None):
    return argv[argv.index(name) + 1] if name in argv else default


if os.environ.get("ALT_FAKE_ARGV"):
    with open(os.environ["ALT_FAKE_ARGV"], "w") as handle:
        json.dump(argv, handle)
if os.environ.get("ALT_FAKE_ENV"):
    with open(os.environ["ALT_FAKE_ENV"], "w") as handle:
        json.dump({"LD_LIBRARY_PATH": os.environ.get("LD_LIBRARY_PATH", "")}, handle)
key = option("--api-key")
if option("--api-key-file"):
    with open(option("--api-key-file")) as handle:
        key = handle.read().strip()
alias = option("--alias", "fixture")
context = int(option("--ctx-size", "4096"))


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def reply(self, status, body):
        data = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def authorized(self):
        return key is None or self.headers.get("Authorization") == f"Bearer {key}"

    def do_GET(self):
        if self.path == "/health":
            return self.reply(200, {"status": "ok"})
        if not self.authorized():
            return self.reply(401, {"error": {"message": "Invalid API Key"}})
        if self.path == "/v1/models":
            return self.reply(200, {"object": "list", "data": [{"id": alias, "object": "model"}]})
        if self.path == "/props":
            return self.reply(200, {"default_generation_settings": {"n_ctx": context}, "chat_template": "{{ enable_thinking }}"})
        self.reply(404, {"error": {"message": "not found"}})

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        self.rfile.read(length)
        if not self.authorized():
            return self.reply(401, {"error": {"message": "Invalid API Key"}})
        self.reply(200, {"choices": [{"message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
                         "usage": {"prompt_tokens": 3, "completion_tokens": 1}})


ThreadingHTTPServer(("127.0.0.1", int(option("--port", "8080"))), Handler).serve_forever()
