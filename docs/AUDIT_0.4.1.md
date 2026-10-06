# Alt 0.4 audit and 0.4.1 corrections

October 6, 2026. This is a source review, executable regression investigation,
dependency advisory scan and local integration audit. It is not an independent
third-party security certification. Feature delivery in the original twelve
work orders does not mean every failure mode or supported machine was validated.

The corrected source passes 59 Rust tests, formatting and strict Clippy. Three
real-terminal walkthroughs, both real-Goose adapter fixtures, portable first-run
and Task walkthroughs, actual SSH/tmux and an older Debian 11 launch pass. Final
archive installation and upgrade/rollback results are recorded in its external
release receipt. A portable-build attempt ran out of storage; after preserving
the tested debug executable, rebuildable debug artifacts were cleaned and the
portable build passed. A fixture-oracle invocation missing the documented Rust
PATH failed; its correctly configured rerun passed. Both attempts are retained.

## Findings corrected

| ID | Priority | Reproduced problem | Correction and regression |
|---|---|---|---|
| A01 | High | A check printed more than the saved-output limit, then `Ran 0 tests`, and exited successfully. The summary was missed and the requirement could appear passed. | Count supported runner summaries while draining both streams, independently of the bounded saved output. Keep at most a 16 KiB summary line and counters. Tests cover a trailing zero-test summary and a real Rust target followed by a zero-test target. |
| A02 | High | A persistent terminal's control client closed before reading its response. `Broken pipe`/connection reset escaped the supervisor loop and terminated the program. | Treat a response transport failure as a client disconnect; bound response writes to 250 ms. A real-process regression abandons one client, reconnects another, confirms the process survives, then stops it normally. |
| A03 | High | Turn completion launched a background Task refresh and performed another verification read synchronously. They competed for the same project lock. Under load, an error dialog interrupted the next form. | Compute verification once in the background refresh and deliver its notice with the result. A multi-threaded regression repeats completion while a draft is open; the real PTY failure is retained. |
| A04 | Medium | A flat directory containing 50,001 files bypassed the documented 50,000-file limit, which was checked only when entering a directory. | Check each file before adding it; honor cancellation during discovery. The boundary regression rejects 50,001, accepts exactly 50,000 even with an empty directory, and checks that a rejected index update preserves the old index. |
| A05 | Medium | Valid MCP SSE using `data:` without a space and multiline data failed as unreadable JSON. Parsing could also accept an event before its terminating blank line. | Decode SSE incrementally, including BOM, comments, CR/LF/CRLF, multiple data lines, unrelated IDs and split UTF-8. HTTP integration plus every-chunk-boundary regressions verify framing; the 1 MiB response bound remains. |
| A06 | Medium | Multiline paste replaced tabs with spaces, breaking Makefile recipes. The editor accepted files up to 65,536 bytes but stopped insertion at 64,000. | Preserve tabs in multiline input; retain single-line normalization; align insertion with the documented 64 KiB limit. |
| A07 | High dependency maintenance | RustSec reported two `lru 0.12.5` soundness advisories, RUSTSEC-2026-0002 and RUSTSEC-2026-0253, and unmaintained `paste 1.0.15` via Ratatui 0.29. | Upgrade Ratatui to 0.30.2, resolving `lru 0.18.5` and removing `paste`; retain only the required frontend features. Add a pinned cargo-audit CI gate that fails on advisory warnings. The strict post-upgrade scan has no findings or warnings. |

A07 establishes affected dependency versions, not a demonstrated exploit in Alt.
The original scan returned exit zero despite informational soundness warnings;
the new gate explicitly uses `--deny warnings`. It does not suppress any advisory.
The database commit is recorded in both scan reports, so “clean” has a dated scope.

The verifier contract is now version 2. Earlier check records lack this marker
and become stale after upgrade; rerun required checks before trusting them. No
old result is rewritten to make it pass. Full access retains arbitrary commands,
network access, installation and external tools with normal host permissions.

## Evidence and validation boundaries

The [audit evidence directory](evidence/audit-2026-10-06) preserves failing
regressions (`*-before.log`), the initial PTY failure (`tui.log`), advisory reports
before/after, and final checks. The [validation manifest](evidence/audit-2026-10-06/validation.json)
identifies the tested executables and outcomes. The old 0.4.0 archive and its
`dist/release-verification.json` receipt remain historical artifacts.
The corrected package is 0.4.1; its final receipt is kept outside the archive at
`dist/alt-0.4.1-verification.json` to avoid a self-referential archive checksum.

Review covered project verification/indexing, subprocess and PTY lifetimes,
the TUI event loop/editor, ACP/MCP boundaries, model/download configuration,
state storage/migrations, workflow packs, setup, CI and packaging. Existing
tests cover the other paths; this audit does not claim fresh live tests of every
optional browser, scanner, runtime or physical GPU.

All tests added here are deterministic and use no model weights. The real Goose
adapter checks use protocol fixtures. No new inference benchmark was run, and
the dependency/TUI changes do not establish improved model reasoning. The
[live model results](LIVE_EVALUATION.md) remain mixed: the stronger repeated
Qwen baseline was 2/12 independent source outcomes on each runtime. Spark and
MiMo exploratory cases are too few to rank them generally; source correctness
and a completed, accurate model turn must be scored separately.

## Remaining work, in recommended order

These are new work orders, not claims of completed implementation. Each needs
its own reviewable change and evidence before it can be marked delivered.

### R01 — Release provenance and mandatory artifact CI

**Priority:** before a public release. **Depends on:** the corrected source passing.

`scripts/package-linux.py --binary` checks version and hashes current source
inputs, but cannot prove an arbitrary same-version binary was built from those
inputs. Current archive reproducibility means repackaging identical inputs,
not two independent compiler builds. CI uses the runner's compiler environment;
the Debian 11 portable builder is currently a separate local path.

- Build the distributed executable in the pinned portable builder in CI and embed
  the commit/source-tree hash, dirty state, toolchain and build target at compile time.
- Require packaging to match that embedded record; reject stale same-version binaries.
- Generate a resolved runtime dependency SBOM, preserve licenses, and attach the
  advisory report and build receipt to the exact tested archive.
- Run install, previous-version upgrade, rollback, integrity and installed-TUI
  tests against that archive on the oldest advertised host and a current host.
- Produce a production signature only in the release workflow with its separately
  configured signing identity. Keep test-key signing clearly labelled.

**Acceptance:** a deliberately stale same-version binary is rejected; a clean
remote CI run identifies the source and exact distributed archive; install/rollback
passes on both hosts. Publication and independent-build reproducibility remain
separate checks. The workflow edits in this audit have been exercised locally,
not on GitHub Actions.

### R02 — Consistent background work and project operation ownership

**Priority:** next implementation batch. **Depends on:** A02/A03.

Some remaining TUI handlers still synchronously read files, history, context or
verification (`workbench.rs`), and project access uses a fail-fast exclusive lock.
Aborting a Tokio handle does not stop an already-running `spawn_blocking` closure.

- Move filesystem/database/index/verification work to a project service with
  explicit queued, running, cancelling and completed states.
- Tag results with the project/task/generation that requested them, so old results
  cannot replace the view after the user switches projects.
- Use cooperative cancellation inside long blocking work; distinguish operations
  that can cancel safely from atomic writes already being committed.
- Keep form drafts intact while background errors arrive; coalesce read refreshes
  and show temporary contention inline instead of interrupting the next action.

**Acceptance:** at least 30 repeated PTY walkthroughs under CPU/I/O pressure,
zero lost drafts or self-contention dialogs, measured input latency (target p95
under 100 ms) and cancellation acknowledgement (target under 2 s). Measure these
on both cloud and reference-class hardware rather than claiming universal timings.

### R03 — Verification that expresses exactly what was proven

**Priority:** next implementation batch. **Depends on:** A01.

Exit zero from an arbitrary configured command establishes that command's result,
not that the user's requested behavior was tested. Summary parsing remains a
heuristic for supported runners. It does not understand every test runner, an
oversized summary line, dynamic dependencies or mutable remote services.

- Distinguish tests, build, lint, health and custom checks in the saved contract.
  Require a positive structured test count only for checks declared as tests.
- Prefer structured runner reports (for example JSON/JUnit/TAP) and record whether
  collection was complete; display “unknown coverage” separately from “zero tests”.
- Identify runner/environment/dependency state more completely, including changes
  inside an existing virtual environment or installed package tree. Current source
  and lockfile hashes do not detect every such change.
- Bind each requirement to a user-reviewable behavioral assertion and keep
  independent acceptance assertions outside model-editable source.
- Distinguish check success, complete task acceptance and model completion prose
  throughout CLI/TUI/export, and preserve the verifier contract in evidence.

**Acceptance:** zero tests, incomplete reports, stale environments, fabricated
summaries, command-only success and altered assertions cannot satisfy a declared
behavioral test requirement. Legitimate build-only tasks remain usable.

### R04 — Measurable small-model assistance

**Priority:** before recommending a model as dependable. **Depends on:** R03.

- Add observed capability profiles for each exact model/template/runtime pair:
  tool JSON, edit accuracy, context handling, tokenizer support and stop behavior.
- Improve one-file-at-a-time repair guidance and feed concise actual check failures
  back to the same selected model. Detect repeated identical failed calls and
  preserve the user's ability to continue or change strategy.
- Select a relevant subset of tool schemas and retrieved code per task. Pin the
  goal and current evidence; count actual tokens where the runtime supports it.
- Evaluate at least 20 representative tasks with five attempts per selected
  configuration, plus held-out tasks that were not used for prompt tuning.
  Include repository setup, migrations, dependency repair and service debugging.
- Compare before/after using identical artifacts and real native windows. Retain
  timeouts, invalid calls, no-edit turns, time/memory and inaccurate completion prose.

**Acceptance:** publish denominators and separate source correctness, complete
turns and accurate claims. Improvements need measured evidence on the unchanged
suite and held-out tasks. Use only explicitly selected uncensored/abliterated
weights for live evaluation; never silently switch providers or models.

### R05 — Real hardware and runtime qualification

**Priority:** before advertising old-PC performance. **Depends on:** R04 harness.

- Test CPU-only machines at 8/16 GB RAM, the GTX 1070 8 GB target, and a newer GPU.
  Record drivers, actual offloaded layers, quantization, native window and KV settings.
- Exercise 4K/8K/16K windows, sustained workloads, allocation failure, cancellation,
  driver/runtime failure, storage pressure and restart without silent fallback.
- Test real LM Studio and private/gated Hugging Face acquisition when an accessible
  runtime and authorized credentials are available; preserve origin/auth checks.
- Offer measured presets and a clear explanation when a model/context does not fit.

**Acceptance:** a published compatibility matrix backed by actual runs and failure
recovery. Cloud CPU results do not satisfy GPU rows. Hardware, credentials and
the previously blocked LM Studio installer remain external acceptance gaps.

### R06 — Recovery, migrations and storage lifecycle

**Priority:** before a stable release. **Depends on:** R02.

- Check schema compatibility before any schema-creating statements; current
  Project/Store open paths perform legacy CREATE statements before migrate checks.
- Make multi-record state updates transactional and test termination between writes.
- Extend existing ENOSPC/interrupted-edit tests to backup, restore, settings,
  model relocation and package installation; exercise concurrent processes.
- Clarify restore scope in the UI and run an unattended restore drill from archived
  state. Keep original imported weights and conflicting user files intact.

**Acceptance:** newer schemas are rejected without modification; every injected
failure leaves either the previous complete state or a recoverable, explained
transition. No “success” notice before durable completion.

### R07 — Parser fuzzing and long-running reliability

**Priority:** before a stable release. **Depends on:** deterministic regressions.

- Fuzz ACP/MCP framing, SSE, archives, tool arguments, UTF-8 edits, path handling
  and LSP ranges with byte/allocation/time bounds.
- Add state-machine tests for edit/undo/restart and persistent job transitions.
- Soak streaming, terminal attach/detach, repeated downloads and interrupted tools;
  track owned processes, file descriptors, memory and disk growth.
- Keep fast deterministic CI separate from scheduled heavier integration/soak runs;
  retain failures instead of retrying until the dashboard is green.

**Acceptance:** no panic, orphaned owned process, unbounded growth or state loss
over the declared corpus/workload; failed cases become permanent regressions.

### R08 — Novice usability and maintainable implementation

**Priority:** alongside the above. **Depends on:** stable workflows.

- Observe people unfamiliar with coding completing connection, model download,
  first repair, verification, error recovery, backup and restore without coaching.
- Replace implementation vocabulary where it prevents a user making a choice;
  show what happened, what was saved, and the next actionable step.
- Split large TUI handlers and compressed workflow blocks into typed services and
  small view/controller functions. Keep adapters, task state and UI independent.
- Test keyboard-only operation, narrow terminals, screen-reader/plain CLI paths,
  non-ASCII paths and code, and failed connections with unsent work.

**Acceptance:** recorded task-completion and recovery results from actual novice
sessions, zero lost drafts, and feature changes with focused tests that do not
depend on rendering unrelated pages. PTY automation alone does not prove usability.

## Definition of release readiness

Treat delivery, correctness, usability, model quality and hardware qualification
as separate gates. Each work order needs an observed failure or explicit
requirement, an implementation, a regression/acceptance check, retained evidence,
and a stated boundary. Passing tests and a clean advisory scan are necessary
evidence; neither is proof that all software behavior is correct.
