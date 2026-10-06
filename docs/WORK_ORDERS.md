# Approved work orders — Alt 0.4 beta

The subsequent [0.4.1 audit](AUDIT_0.4.1.md) records corrected defects and new
reliability/release work orders. “Delivered” below describes the feature scope;
release readiness and unperformed hardware acceptance are separate gates.

The owner authorized implementation of the complete [next-release proposal](NEXT_RELEASE.md),
including the items originally grouped under 0.4, 0.5 and hardware/runtime work.
This Linux beta combines those product features. Acceptance of exact hardware,
credentials and model quality is recorded separately from implementation.

Full access continues to offer arbitrary commands, networking, installation and
external tools with normal host permissions. All real inference evaluations use
an explicitly selected uncensored/abliterated artifact. No standard/cloud fallback
is authorized. Windows/macOS/ARM packaging, collaborative agents, fine-tuning and
abliteration pipelines were explicitly later work in the proposal and remain so.

## WO-01 — Independent acceptance and continuous integration

**Depends on:** nothing. **Status:** delivered; repeated baseline and strengthened reassessment complete.

- [x] Pin the Rust toolchain/lockfile and real Goose engine; provide repeatable cloud setup.
- [x] Add CI for compilation, Rust tests, formatting, Clippy, PTY workflows and both provider adapters.
- [x] Seed Python/Rust/JavaScript projects for a feature, multi-file repair, missing dependency,
  module configuration, HTTP behavior and a local application-security defect.
- [x] Keep independent outcome assertions outside the editable project; check their integrity.
- [x] Exercise every broken seed and known repair, without an inference model.
- [x] Freeze the tested executable; retain every live attempt, output, source result,
  tool call, failure, time and sampled process memory. Distinguish external-server RAM.
- [x] Add optional required-check plans and older-decision history to context evaluations.
- [x] Strengthen oracles against incomplete repairs and preserve separate reassessments of saved source.

**Acceptance:** every seeded defect fails its oracle and every known repair passes.
A model's prose/process exit cannot override that oracle. All repeated attempts,
including timeouts and an interrupted development run, remain in evidence.
**Implementation:** `.github/workflows/verify.yml`, `scripts/setup-cloud.sh`,
`scripts/acceptance_projects.py`, `scripts/live_acceptance.py`.
**Evidence:** [setup log](evidence/v4/setup.log), [current validation](IMPLEMENTATION.md).
CI itself has not been run on a remote GitHub runner; its local command sequence has.

## WO-02 — Task verification plans

**Depends on:** 01. **Status:** delivered and tested.

- [x] Persist named outcomes, their descriptions and exact configured checks.
- [x] Show unrun, failed, cancelled, timed out, zero-test and stale results distinctly.
- [x] Require every configured requirement to pass; empty requirements are unverified.
- [x] Bind results to source/permissions, command, lockfiles, executable and environment.
- [x] Record observed runner versions where available; do not confuse them with a rerun.
- [x] Add direct CLI and Task-page plan/rerun/status actions.
- [x] Show computed verification after model turns in both the TUI and headless output.
- [x] Return syntax feedback and actual check names to help small models recover.

**Acceptance:** changing files, a check command or its environment invalidates a
previous pass; one unrelated green command cannot complete all requirements.
**Implementation:** `src/project_services.rs`, `src/project.rs`, `src/sandbox.rs`,
`src/toolbox.rs`, `src/engine.rs`, `src/main.rs`, Task/Workspace TUI.
**Evidence:** `tests/project_services.rs`, `tests/project_workflow.rs`, both Goose
fixtures and `scripts/smoke_task_tui.py`. Unknown test counts stay unknown.

## WO-03 — Interactive terminal and durable jobs

**Depends on:** 02. **Status:** delivered and tested.

- [x] Add PTYs, input/control/paste forwarding, screen parsing and resize.
- [x] Add attach/detach, logs, stop/restart, service health and explicit time budgets.
- [x] Let users choose Alt-owned or persistent lifetime; expose choices in the TUI.
- [x] Save job/process identity and reconcile after restart without signalling a reused PID.
- [x] Bound output/logs/input; keep Stop responsive when the child stops reading.
- [x] Drain and preserve the final terminal tail when a command exits.

**Acceptance:** input reaches a real program; explicitly persistent jobs survive
Alt restart; owner death stops only owned jobs; timeout/stop kill owned descendants;
large output and input backpressure do not freeze the supervisor.
**Implementation:** `src/jobs.rs`, Jobs CLI and `src/tui/workbench.rs`.
**Evidence:** four real-process tests in `tests/jobs.rs`, extended PTY walkthrough,
[actual SSH/tmux walkthrough](evidence/v4/ssh-tmux.json).

## WO-04 — A complete novice workspace

**Depends on:** 02,03. **Status:** delivered and tested.

- [x] Keep first-run setup usable without a model/config file; preserve failed forms/prompts.
- [x] Add Files browsing/filtering, text search, go-to-line views, editing and new-file preview.
- [x] Expose grouped diffs, checkpoints, task undo and conflicts.
- [x] Add Jobs and Context pages; retain Home task starters and a complete keyboard route.
- [x] Add Settings wizards for tools, extensions, findings, language servers,
  storage/recovery, model cache, runtime tuning and measurement.
- [x] Explain recovery actions and show objective, activity, actual result and next step.
- [x] Add plain terminal chat and reviewable redacted diagnostics.
- [x] Test narrow terminals, normal PTY, real SSH, and SSH with tmux.

**Acceptance:** first-time setup/model selection, manual project change, direct
verification, undo, error recovery and persistent terminal operation complete
through labelled UI actions without TOML edits. Installed-package walkthrough also passes.
**Evidence:** `scripts/smoke_tui.py`, `smoke_task_tui.py`, `smoke_workbench_tui.py`,
`smoke_terminal_hosts.py`, Rust view/input/recovery tests and [screenshots](screenshots/v4).
The bounded editor supports UTF-8 files up to 64 KiB; Full access terminal editors remain available.

## WO-05 — Incremental project memory and context

**Depends on:** 02. **Status:** delivered; managed 4K/8K/16K retrieval/context measurements recorded.

- [x] Separate 50,000-path exploration from stricter edit/check snapshot limits.
- [x] Reuse unchanged index entries, reconcile edits/deletions and cancel without partial commits.
- [x] Use Tree-sitter chunks/repository maps for Rust, Python and JavaScript.
- [x] Add pinned requirements, full-text retrieval of old decisions and visible actual context.
- [x] Prioritize current computed evidence over contradictory historical notes.
- [x] Count managed memory with the selected llama.cpp tokenizer when available;
  label estimates and distinguish memory from the full engine/native window.
- [x] Add optional installed-LSP references and reviewed UTF-16 rename with checkpoints.

**Acceptance:** changed/deleted files cannot return stale excerpts; old relevant
decisions and current evidence survive bounded histories. The 5,001-file measurement
reuses all unchanged entries and updates exactly the changed/deleted files.
**Evidence:** `tests/project_services.rs`, `tests/language.rs`, actual rust-analyzer
rename plus independent Cargo check; [retrieval timings](evidence/v4/retrieval.json).
4K/8K/16K character-memory regressions are distinct from native-context live tests.
Embeddings remain excluded because no measured benefit justifies their resource cost.

## WO-06 — Storage, migration and crash recovery

**Depends on:** 02. **Status:** delivered and tested.

- [x] Version schemas; create recoverable pre-migration SQLite copies; reject newer state.
- [x] Back up consistent SQLite databases and checksummed bounded state archives.
- [x] Restore only into new/empty folders; reject malformed/extra/linked archive entries.
- [x] Show usage and preview retention; archive old user-archived conversations before removal.
- [x] Restore retained history without overwriting existing sessions.
- [x] Preserve corrupt settings, including runtime settings separately from project preferences.
- [x] Inject ENOSPC and abrupt process exit before/after edit/undo rename boundaries.
- [x] Recover journal states using hashes without replaying side effects or overwriting conflicts.

**Acceptance:** failure preserves original data or a precise recoverable state;
failed migrations leave old schema/data intact; retained history restores exactly.
**Implementation:** `src/schema.rs`, `src/storage.rs`, `src/store.rs`, `src/project.rs`.
**Evidence:** `tests/faults.rs`, `tests/storage.rs`, migration and TUI recovery tests.
Backups cover Alt state, not the project's whole filesystem or downloaded weights.

## WO-07 — Versioned workflows and evidence-backed findings

**Depends on:** 02,03. **Status:** delivered and tested with actual optional tools.

- [x] Declare prerequisites/inputs/parsers for build, Git, HTTP, browser, Semgrep,
  pip-audit and npm audit; keep heavy dependencies optional.
- [x] Keep arbitrary Full access terminal commands available beyond the packs.
- [x] Make Git commits include only chosen paths, preserving unrelated staging.
- [x] Save every attempt; reject missing tools, malformed/incomplete output and empty scans.
- [x] Record findings, reproduction, evidence IDs, confidence/severity, fixes and linked retests.
- [x] Expose Settings workflows and JSON/Markdown/SARIF export.

**Acceptance:** deliberately broken browser behavior fails, corrected behavior
passes; real scanners/auditors detect seeded issues; retests link new execution
records; a disconnected service cannot produce a passing result.
**Evidence:** `tests/packs.rs`, [browser/Semgrep records](evidence/v4/browser-scanner.json),
[dependency-auditor records](evidence/v4/dependency-auditors.json), retained browser
screenshot. The earlier zero-file Semgrep false-pass attempt is preserved separately;
its acceptance failure led to the nonempty-scan check.

## WO-08 — External MCP manager

**Depends on:** 03,07. **Status:** delivered and tested.

- [x] Support local stdio and remote Streamable HTTP JSON/SSE sessions.
- [x] Negotiate protocol, bound requests/responses and discover paginated inventories.
- [x] Use environment-name authentication; prevent forwarding remote credentials to another origin.
- [x] Default connections to disabled; expose only selected tools (maximum 16).
- [x] Preserve client session state and reconnect after failure without replaying actions.
- [x] Route extension requests through Alt's actual one-use approval broker and selected access mode.
- [x] Provide CLI and TUI add/probe/select/disable/remove flows.

**Acceptance:** selected inventory appears in real Goose; denied calls have no side
effect; allowed calls return actual records; remote auth/session IDs/SSE and stdio
state/reconnect work. **Evidence:** `tests/extensions.rs`, both expanded
`scripts/smoke_goose.py` fixtures, MCP wizard PTY.

## WO-09 — Model acquisition and cache management

**Depends on:** 04. **Status:** delivered; credentialed private-Hub acceptance externally untested.

- [x] Use exact HTTPS Hub origin authentication through `HF_TOKEN`; preserve normal TLS trust.
- [x] Recognize complete numbered multipart GGUF sets and pin revision/size/SHA256 for every piece.
- [x] Preflight remaining set disk space, resume partial pieces and atomically publish only verified sets.
- [x] Relocate by copy/hash/switch; retain original copies plus a receipt.
- [x] Remove unreferenced managed library weights while preserving all imported originals.
- [x] Expose Models/cache/authentication help through the TUI and CLI.
- [ ] Exercise a credentialed private/gated repository (no account token was supplied).

**Acceptance:** an interrupted/corrupt/missing part cannot become ready; retries
resume; relocation preserves integrity; imported originals survive removal.
**Evidence:** eight download/auth/import tests including multipart fixtures and prior
actual public Hub download provenance. No `HF_TOKEN` was available for a private
repository test. Retained old relocation copies require explicit user cleanup.

## WO-10 — Hardware/runtime compatibility and model quality

**Depends on:** 01,09. **Status:** delivered; real CPU evaluations complete;
physical GTX 1070 and LM Studio acceptance remain external.

- [x] Expose CPU threads, GPU layers, batch/cache/flash-attention settings without model substitution.
- [x] Reject requested GPU operation when the selected runtime reports no accelerator.
- [x] Correct container RAM estimates for reclaimable inactive file cache; preserve anonymous/tmpfs/active working-set costs.
- [x] Add measured load/generation reports, available RSS/VRAM, cancellation and allocation recovery.
- [x] Provide pinned CUDA 12 / Pascal 6.1 build recipe and CPU fallback choice.
- [x] Run repeatable exact-artifact coding acceptance on actual llama.cpp and actual Ollama.
- [x] Distinguish protocol fixtures, throughput, native-tool probes and independently verified task quality.
- [x] Correct the pinned engine’s Ollama output-limit compatibility and assert actual wire fields.
- [x] Distinguish Alt/engine context budgets from native server/tag windows.
- [ ] Validate CUDA recipe, fit/performance and OOM behavior on a physical GTX 1070.
- [x] Refresh exact Spark-X2.5/MiMo derivative discovery with pinned revisions, file checksums and compatibility caveats.
- [x] Measure exact 1.7B Spark and 9B MiMo derivatives: load, native tools, two coding tasks, memory, timeouts and prose.
- [ ] Validate a real LM Studio instance (official installer blocked here by HTTP proxy 403).

**Acceptance:** every measurement identifies artifact, runtime, context and machine;
CPU results never become GPU claims. Fixture allocation/cancellation tests pass.
The original matrices scored **3/12** (llama.cpp) and **2/12** (Ollama); strengthened
assertions score both at **2/12**. Original reports and reassessments remain visible.
Final native-context retests and Spark/MiMo explorations include passes and timeouts.
None is promoted as a generally reliable autonomous coder. See the [complete live
report](LIVE_EVALUATION.md) for every cohort and exact scope.

## WO-11 — Reproducible release, installation and rollback

**Depends on:** 01,06. **Status:** delivered; signatures, reproducibility and installed upgrade/rollback passed.

- [x] Build against pinned Debian 11 ABI and record measured GLIBC/component requirements.
- [x] Normalize archive order, ownership, timestamps and modes; record source/dependency/compiler hashes.
- [x] Include all-file integrity manifest, licenses, provenance and optional configured-key signatures.
- [x] Verify before mutation, atomically install and retain the previous distinct executable.
- [x] Preserve rollback through repeated installation; test real 0.3 → 0.4 → 0.3 → 0.4 state compatibility.
- [x] Verify changed payload rejection leaves installed bytes intact.
- [x] Exercise the installed binary and TUI outside the development checkout.

**Acceptance:** deterministic archives match for identical inputs; test-key signature
verification rejects tampering; prior-version rollback opens preserved state;
packaged workflows pass. **Evidence:** `scripts/smoke_package.py`,
[final installed acceptance log](evidence/v4/final-installed-acceptance.log), current validation record.
Production signing credentials were not supplied; the final local artifact is unsigned
and unpublished. Archive reproducibility is not a cross-toolchain compiler reproducibility claim.

## WO-12 — End-to-end evidence and handoff

**Depends on:** all preceding orders. **Status:** delivered; external acceptance gaps remain explicitly open.

- [x] Publish an operator/workspace guide with exact controls, limits and recovery steps.
- [x] Preserve prior-version implementation/research and failed evaluation attempts.
- [x] Distinguish application acceptance, model failures and external validation gaps.
- [x] Finish all live cohorts, manual prose-accuracy reviews, source reassessment and actual native-context measurements.
- [x] Save final validation, artifact/provenance and reusable cloud configuration draft.
- [x] Stop test-owned inference/services and verify no active test jobs remain.

Completion does not imply that every arbitrary model/program is reliable. Product
features are reviewable locally; GPU/private-Hub/commercial-runtime checks require
the corresponding hardware, credentials or installed server. Those gaps remain
explicit rather than being replaced with simulated passes.
