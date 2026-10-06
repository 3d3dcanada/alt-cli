# Implementation and verification — Alt 0.4.1 beta

October 6, 2026. The complete approved product scope is tracked in the twelve
[work orders](WORK_ORDERS.md). This release adds the workspace, terminal, verification,
recovery, extensions and runtime controls originally proposed across 0.4 and 0.5.
The [workspace guide](WORKSPACE_GUIDE.md) explains operation; the
[0.3 implementation](IMPLEMENTATION_0.3.md) remains as history.
The [0.4.1 audit](AUDIT_0.4.1.md) adds six reproduced correctness fixes and a
dependency upgrade, with remaining release, reliability and usability work orders.

## Delivered behavior

| Area | What is implemented | Evidence |
|---|---|---|
| Novice workspace | Eleven pages, labelled setup/preferences, file browser/search/editor, grouped diffs/undo, jobs, visible context, preserved forms, plain terminal chat | Three PTY walkthroughs, view/input/recovery tests, actual SSH and SSH/tmux, installed-package walkthrough |
| Independent completion | Named requirements mapped to checks; all required checks must pass on current source, command and recorded environment; unconfigured, zero-test, failed, cancelled and stale are distinct | Verification regressions, independent Python/Rust/JavaScript acceptance oracles, headless/TUI computed results |
| Terminal and jobs | Real PTYs, input/paste/resize, attach/detach, durable ownership and PID identity, chosen lifetime/timeout, stop/restart/logs/HTTP health | Four real-process tests, input backpressure/large output, abrupt owner exit and persistent restart, TUI terminal walkthrough |
| Memory and language tools | Incremental index, Tree-sitter chunks/maps, old decision retrieval and pins, visible actual memory, managed token counting, optional installed-LSP references/reviewed rename | Index/context regressions, 5,001-file measurement, actual rust-analyzer rename and independent Cargo check |
| Recovery and storage | Schema migrations with recovery copies, consistent backups, checksummed new-directory restore, retention archives/restore, diagnostics, interrupted-edit journal reconciliation | ENOSPC/process-death injection, migration/archive corruption/conflict tests, oversized-retention preservation |
| Workflow packs and findings | Versioned build, Git, HTTP, browser, Semgrep, pip-audit and npm audit; observed findings/reproduction/retest; JSON/Markdown/SARIF | Real Chromium, Semgrep and dependency auditors on broken/repaired fixtures; selected-path Git and malformed/empty-scan regressions |
| External MCP | stdio and Streamable HTTP JSON/SSE, protocol/session negotiation, selected inventory, origin-bound authentication, stateful reconnect without replay | Real Goose with an external tool, approval/denial effects, HTTP/SSE/session/auth and stdio reconnect tests |
| Models and runtime | Origin-scoped Hub authentication, verified resumable multipart GGUF, disk preflight, cache relocation/cleanup, CPU/GPU/cache controls, benchmark reports, Pascal build recipe | Multipart/auth/cleanup fixtures, actual public artifact, GPU-unavailable/allocation/cancellation tests, actual llama.cpp and Ollama inference |
| Distribution | Pinned Debian 11 build, measured ABI, normalized archives, input/dependency provenance, every-file integrity, optional signatures, atomic executable upgrade/rollback | Older-system/container checks, actual prior-version migration/rollback, tampered update rejection and installed TUI |

Full access retains arbitrary commands, network access, installations and external
tools with normal host permissions. Guided changes and Review only are optional
user-selected modes. The seven native tools remain `list`, `read`, `search`,
`remember`, `edit`, `run_check` and `terminal`; selected external tools supplement
them. The separate Goose ACP engine is not the product UI and is not bundled.

## Validation

The 0.4.1 source passes **59 Rust tests**, formatting and strict Clippy (0.4.0 had 52).
The new regressions cover disconnected terminal clients, discovery limits,
SSE framing, tab-preserving paste, trailing zero-test summaries, and a concurrent
TUI refresh. A strict cargo-audit scan of the updated lockfile reports no advisories
or warnings against its recorded database revision. These tests cover
behavior, interruption, persistence, migrations, container-memory cache accounting and actual subprocess/protocol
boundaries. The three PTY scripts exercise first-run setup, task/check/access flows,
and the full file/context/extensions/jobs workspace. Both real-Goose provider
fixtures pass, including selected external tools and plain-terminal approval.
Deterministic provider fixtures have no model weights and do not measure reasoning.

The six independent acceptance projects each reject their seeded defect and accept
the known repair. Four incomplete repairs are also rejected, and saved baseline
source was reassessed without rewriting the original scores. Actual optional browser/scanner/auditor tests and actual SSH/tmux
have separate records. The 5,001-file index took 5.795 seconds initially, 0.030 seconds
unchanged and 0.031 seconds for one edit plus one deletion on this cloud filesystem.
These are workload-specific observations, not universal performance promises.

See the [0.4.1 audit manifest](evidence/audit-2026-10-06/validation.json) for current
binary/check provenance and the historical [0.4.0 validation manifest](evidence/v4/validation.json),
[evidence directory](evidence/v4), [screenshots](screenshots/v4), and
[live evaluation report](LIVE_EVALUATION.md) for exact commands, binaries, outcomes
and retained failed attempts. Remote GitHub Actions has not been run; its local
command sequence has. The final local package is unsigned and unpublished; test-key
signature acceptance is distinct from production release signing.
The 0.4.1 verifier contract invalidates older check evidence; rerun requirements
after upgrade. Live-model measurements below remain the earlier measured builds.

## Model quality is a separate result

Live inference uses explicitly selected, SHA256-verified uncensored/abliterated
Qwen3-4B Heretic, Spark-X2.5 1.7B and MiMo V2.6 Distill Qwen 9B artifacts. The application does not silently change the model or provider. Live
acceptance is scored by separate assertions against actual source, including
timeouts and unchanged files, rather than by the model's answer or process exit.
The model has repeatedly printed confident completion claims without applying a
correct edit. Alt's computed check status remains independent and visible.

The full retained matrix and final-build verification/context follow-ups are
reported in [LIVE_EVALUATION.md](LIVE_EVALUATION.md). One native tool-call probe,
one successful edit, or a larger context setting does not establish broad coding
reliability. CPU cloud results do not establish GTX 1070 performance.

## Explicit limits and external acceptance

- Linux x86_64 is the packaged target. The frontend's measured minimum GLIBC is
  recorded in `PLATFORM.json`; managed Goose requires 2.28 and the selected official
  llama.cpp archive requires 2.34. An older frontend-capable host needs a compatible
  custom runtime or external server; preflight explains this before download.
- No physical GTX 1070 or credentialed private-Hub repository was available.
  The official LM Studio installer request was blocked by the environment proxy
  (HTTP 403), so no real LM Studio instance was tested. The CUDA 12/Pascal 6.1
  recipe is implemented but unvalidated on that card. GPU speed, fit and real GPU
  OOM recovery remain unmeasured. Exact Spark-X2.5 and MiMo derivatives have
  separate exploratory results in the live report; they are not promoted as generally reliable profiles.
- The built-in editor supports 64 KiB UTF-8 files. Index discovery supports 50,000
  paths and 256 MiB text; edit/check snapshots have stricter 4,096-file/64 MiB bounds.
  Full access commands can operate beyond these bounded native workflows.
- Checkpoint undo covers Alt file edits, not arbitrary shell effects, databases,
  installations, remote systems, ACLs or extended attributes. Evidence does not
  capture excluded credential/configuration files or every changing external dependency. Backups cover Alt state, not all
  project files or model weights; exported evidence/backups are not encrypted.
- Managed token counting covers Alt's memory block. Engine prompts, tool schemas
  and conversation consume the native window too. Retrieval preserves useful disk
  history without enlarging a model's real context or guaranteeing its recall.
- Heavy browsers, scanners, language servers and runtimes stay optional. External
  MCP supports stdio and Streamable HTTP, not the old standalone SSE transport.
  Rename supports in-project text edits, not LSP resource creation/deletion/moves.
- Windows/macOS/ARM distribution, collaborative agents, training/abliteration and
  embeddings remain the explicitly deferred scope from the approved proposal.
