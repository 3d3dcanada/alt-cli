# Disposable MCP boundary reproductions

Audited source: `fa0c8e4130ec4d13c59193436679c4e059c22dee` (beta 3).
The two original JSON reports retain the initial observations. The reusable
driver was then rerun successfully against the audited executable; its separate
`replay-report.json` confirms both defects and completed helper cleanup.

Files safe to retain: this README, `reproduce.py`, both `mcp-*-fixture.py` files,
`policy-report.json`, and `helper-report.json`. There are no credentials, model
weights, process artifacts, or user state in this bundle.

Run with the audited executable, choosing a NEW output report path:

```bash
python3 docs/evidence/final-audit-2026-10-08/mcp/reproduce.py \
  --alt /workspace/alt-cli/target/debug/alt \
  --output /tmp/alt-mcp-boundary-recheck.json
```

Each invocation creates a private temporary directory below `/tmp` and a separate
Alt state directory. The marker fixture only writes its disposable marker and
returns a valid MCP initialization and empty inventory. No model, network server,
or normal user configuration is used. The temporary state is removed afterward.

The helper fixture starts one child that ignores SIGTERM and sleeps for at most
30 seconds. The driver records whether it remains running after MCP discovery
returns, sends SIGKILL to that recorded helper (checking process start identity),
and reaps it through Linux's child-subreaper facility. All CLI invocations have a
10-second timeout. Do not run the helper fixture directly: the driver owns its
cleanup. Output contains only fixture results and the executable path.

Expected after fixing the bugs:

- Review-only and Guided discovery fail BEFORE launching the marker fixture;
  `host_marker_written` is false in both cases.
- Trusted discovery succeeds and writes the marker.
- Trusted helper discovery succeeds, with `survived_after_probe_returned` false.
- `cleanup_running` is false.

Original observations: Review-only, Guided and Trusted all executed the marker
fixture, and the helper was still running (state `S`) after discovery returned.
The original helper was immediately killed after recording that evidence.
