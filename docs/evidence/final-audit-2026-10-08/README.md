# Final audit evidence

These are bounded reproductions against beta 3 (`fa0c8e4`), collected for the
[final audit](../../FINAL_AUDIT_2026-10-08.md) and its
[work orders](../../FINAL_PASS_WORK_ORDERS.md).

- `correctness/`: real CLI verification, environment, lock and configuration
  replays; a Rust checkpoint harness and narrowly targeted C fault interposer;
  exact build identity and before/after receipts.
- `ux/`: actual PTY scripts, terminal screens and JSON outcomes for connection
  editing, draft loss, keyboard entry, small layouts and canceled inventory.
- `harness/`: replay against the current Rust library demonstrating missing
  middle-turn constraints and irrelevant explicit-file-only retrieval.
- `runtime/`: missing selected-runtime fallback, incomplete imported GGUF set
  and authenticated Ollama mismatch. These fixtures use no real weights.
- `mcp-policy/` and `mcp-cleanup/`: bounded host-marker and helper-lifecycle
  probes. `mcp/` contains their reusable driver and fixtures, original receipts,
  and a successful rerun in `replay-report.json` with helper cleanup confirmed.
  No model escape or remote exposure is claimed.
- `delivery/`: disposable retention/backup and partial-installation reproductions
  using genuine prior/current binaries. Sparse files and installations were removed.
- `targeted-existing-tests.log`: 13 compact-context and three PC-readiness tests
  passed. Their success does not cover the newly demonstrated scenarios.

No normal user project or model was used. Evidence scripts may contain the
measured cloud paths; inspect their arguments and use new disposable output
folders when reproducing. Compiled binaries, mutable databases, model weights,
bytecode and sparse payloads are excluded. Runtime/PTY observations are not new
model-quality successes. Findings remain unfixed in the audited baseline.

`MANIFEST.json` hashes the retained files, excluding the manifest itself.
