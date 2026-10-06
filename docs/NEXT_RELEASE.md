# Proposed next releases: completeness and verification

This historical proposal is the approved scope for the 0.4 beta implementation.
Track delivery in [WORK_ORDERS.md](WORK_ORDERS.md) and measured results in
[IMPLEMENTATION.md](IMPLEMENTATION.md). The original rationale follows.

This is a proposal following a review of the 0.3 source and saved evidence, not a
claim that the features below are implemented. The existing 21 Rust tests, PTY
tests, protocol fixtures and small live Heretic evaluations establish useful
paths, but do not establish broad coding reliability or GTX 1070 performance.
All additional live inference evaluations must use explicitly selected
uncensored/abliterated checkpoints. Full access remains available for arbitrary
commands, networking, installs and external tools with normal host permissions.

## 0.4: dependable work on real projects

### 1. Verification plans and a realistic acceptance suite

Make a task's completion requirements explicit: the requested behavior, relevant
checks, preserved behavior, and remaining work. Track each required check and the
source revision it verified; one unrelated passing command must not complete the
whole task. Include relevant tool/runtime versions and dependency lock hashes in
evidence. Report zero tests separately from a meaningful test run when a supported
runner exposes its test counts.

Start with disposable Python, Rust and JavaScript projects covering a new feature,
multi-file repair, missing dependency, broken configuration, HTTP behavior, and a
seeded application-security defect. Preserve independent assertions outside the
model's editable project. Repeat each selected model/runtime configuration, retain
failures, and score completed requirements, evidence accuracy, invalid calls,
time, peak RAM and VRAM where available. A model's answer is never the verifier.
Natural-language resumed explanations require accuracy checks as well as a
successful process exit; 0.3 demonstrated why these are separate outcomes.

Acceptance: each fixture has an independent outcome oracle and its failure cases
are exercised; reports include exact model artifact, runtime, context and settings;
aggregate results include every attempt, not just eventual successes.

### 2. Interactive terminal and durable jobs

The current runner supplies no stdin, caps commands at ten minutes, and terminates
the owned process group at completion. It is not a general interactive terminal
or persistent development-server manager.

Add a PTY terminal pane, input forwarding, resize handling and attach/detach.
Add explicit jobs for builds, servers, long checks and downloads, with live logs,
status, health checks, stop/restart and configurable time budgets. Let a user keep
a chosen job running when leaving a conversation, while making its ownership and
shutdown behavior clear. Reconcile saved job records with actual processes after
restart; a saved PID alone is not proof that the same process is still running.

Acceptance: interactive programs receive input; servers remain available to later
HTTP/browser checks; cancellation and reconnect work; cleanup tests distinguish
session-owned jobs from jobs the user explicitly chose to keep.

### 3. TUI completion for people who do not code

Add a project file tree, searchable file viewer, editable scratch/input areas,
grouped multi-file diffs, and a clear change/review history. Provide task starters
such as Build an app, Fix an error, Explain this project, and Check its security.
Turn failures into applicable actions: reconnect, select a smaller model, install
a missing dependency, retry a download, or inspect the failed check.

Show the user's objective, what Alt is doing, the actual result, and the next
useful action together. Add a diagnostics export that the user can inspect, with
credentials redacted by default. Keep keyboard operation complete, avoid color-only
status, offer a plain-text mode, and test narrow terminals, SSH and tmux.

Acceptance: fresh-user walkthroughs complete setup, model selection, a project
change, verification, undo and recovery without editing TOML or knowing a CLI
command. Automated terminal tests cover the same paths and interrupted operations.

### 4. Crash recovery and release automation

Add CI for build, formatting, Clippy, Rust tests, terminal workflows and both
provider fixtures. Keep live model suites opt-in or scheduled with explicit
artifact selection. Add fault injection for disk-full/write failures, abrupt
termination during edits and undo, damaged settings/state, stalled downloads,
provider disconnects, oversized output and competing filesystem changes.

Introduce versioned state migrations with recoverable backups, task backup/restore,
configurable history retention and a storage-usage view. Make update installation
atomic, retain a previous executable for rollback, and generate reproducible
archives plus release provenance. Add release signing when release keys are
configured; pinning dependencies alone does not make a signed/reproducible release.

Acceptance: injected failures preserve user files or expose a precise recoverable
state; no false passing check or unintended action replay; upgrade/rollback and
state restoration work against supported prior-version fixtures. Validate the
packaged artifact independently of the development checkout.

## 0.5: stronger tools and useful long-running project memory

### 5. Tool packs, external extensions and findings

Add versioned tool packs for build/test/debug, Git, HTTP/application checks,
browser interaction and application-security review. A pack declares prerequisites,
inputs, outputs, parser and evidence format, so a small model need not invent
command syntax. Start with tools such as Semgrep, language-specific dependency
auditors and browser checks on disposable test applications. Keep heavier browser
and scanner dependencies optional on older computers.

Add an MCP connection manager with stdio and supported remote transports,
authentication configuration, capability discovery, health checks and reconnect.
Expose a small relevant tool set to the model at a time; allow user-selected
extensions and continue to offer the arbitrary Full access terminal.

Git support should explain existing changes, create user-chosen branches, show
diffs, stage selected changes and prepare commits without sweeping unrelated work
into them. Add structured findings with observed evidence, reproducible steps,
severity/confidence, proposed fixes and retest results. Export Markdown, JSON and
SARIF where the finding type supports it.

Acceptance: parsers are tested against success, failure and malformed-output
fixtures; browser checks detect deliberately broken behavior; findings link to
actual tool records; disconnected external tools cannot report a fabricated pass.

### 6. Incremental retrieval and transparent context

The current search rebuilds its FTS index from the entire bounded project on each
query. Replace this with incremental indexing, change detection, cancellation and
recovery after watcher overflow. Separate index limits from the stricter edit and
check-snapshot limits so large repositories can still be explored.

Add syntax-aware chunks, a compact repository map and optional language-server
references/rename where supported. Add a Context view showing the actual current
brief, required decisions, current evidence, selected excerpts and omitted history.
Offer pinned requirements and retrieval of older relevant decisions, not just the
latest few. Use tokenizer-aware accounting where available and visibly distinguish
estimates otherwise. Introduce embeddings only if measured retrieval gains justify
their RAM/CPU cost.

Acceptance: large-project searches stay responsive; changed/deleted files do not
return stale excerpts; old relevant decisions survive long sessions; contradictory
historical evidence cannot replace the current verified result. Measure this at
4K/8K/16K context without presenting disk memory as an enlarged native window.

## Hardware/runtime work in parallel with 0.4–0.5

### 7. Model fit and actual backend compatibility

Add a user-invoked hardware benchmark and exportable compatibility report. Measure
load time, generation speed, completed tasks, peak RAM/VRAM and out-of-memory
recovery for exact model/quantization/context combinations. For the reference
GTX 1070, validate a Pascal-compatible CUDA 12 build; keep CPU operation available
and offer other acceleration only after testing. Hardware tuning should recommend
changes and show their effect, without silently changing the selected model.

Test actual inference on llama.cpp and Ollama, then LM Studio and other requested
servers. Protocol fixtures are not evidence that every server implementation or
chat template works. Expand per-profile capability results beyond arithmetic to
editing, malformed-call recovery and multi-step work.

Add private/gated Hub authentication, verified multipart GGUF downloads,
disk-space preflight, cache relocation/cleanup and clearer compatibility labels.
Continue Spark-X2.5 and MiMo discovery by exact variant; verify an uncensored
derivative before including it in live tests. Keep generic endpoint/model IDs
available even where there is no validated local profile.

Acceptance: measured results identify the physical machine and exact artifacts;
GPU initialization/allocation failure has a useful recovery path; no CPU-only
cloud result is labelled a GTX 1070 result; multipart downloads promote only when
all required pieces verify; cache cleanup never deletes an imported original.

## Delivery order and completion criteria

1. Put the existing checks in CI and add realistic acceptance fixtures plus
   task-level verification plans.
2. Build PTY/jobs and novice recovery flows against those fixtures.
3. Complete crash/migration/update coverage and ship a measured Linux beta.
4. Add browser/Git/security/MCP packs and incremental context improvements.
5. Promote exact model/runtime/hardware combinations as their measurements arrive.

Windows/macOS, ARM packages, collaborative multi-agent execution, fine-tuning and
abliteration pipelines remain later work. They are valuable possibilities, but
would add several independent compatibility and resource problems before the
current Linux product is thoroughly exercised.

A release is ready when its declared user journeys pass from the installed
package, crash/recovery cases have understood outcomes, required checks refer to
current source, and live task success rates and hardware limits are published
honestly. No finite suite can certify arbitrary programs or all model behavior.
