# Correctness and data integrity audit, 2026-10-08

Audited checkout `fa0c8e4130ec4d13c59193436679c4e059c22dee`, clean debug binary labeled `v0.6.0-beta.3`, source SHA256 `54bb6215b1e87579f3534c347f38019cf569e91386237352e314ba5d025976f1`.

No repository source changes, no weights, no full setup rerun. Read AGENTS.md. CLI repros use new disposable projects/state only. The checkpoint harness links the current existing Rust rlib; its C interposer deterministically schedules an editor write at the existing temporary-file fsync boundary, before replacement. This is fault injection demonstrating a real external-writer interleaving, not a statistical reproduction of its frequency.

## Findings in priority order

### A1. P1: external edits between the conflict check and replacement are lost

`src/project.rs:999` validates the current content before writing the durable applying record and staged image. `src/project.rs:968` then replaces the target with no retained image of the displaced file. The project lock (`src/project.rs:174`) only coordinates cooperating Alt operations within the same data folder; external editors do not participate. Undo has the same gap at `src/project.rs:1035` / `1040`.

Repro: after the content check and while Alt syncs its staged file, the interposer writes `USER_EDIT_AFTER_STALE_CHECK` into the existing working file. Apply succeeds and reports `after`; Undo succeeds and reports `before`; the intervening user's version is absent from both the working file and saved before/after checkpoints. Receipt: `race-checkpoint-repro.json`. Source: `checkpoint.rs`, `concurrent_edit.c`.

Work order: define and document attainable conflict guarantees; retain displaced versions in a recovery journal; detect and surface observed concurrent changes; avoid destructive recovery when more external changes occur. Coordinate Alt writers even across data roots if those are allowed to share a project. Do not claim a generic filesystem compare-and-swap or promise protection against all non-cooperating in-place writers. Acceptance: deterministic in-place and rename-based external writes at apply, delete, undo, and batch-undo boundaries; existing unsaved/uncommitted content remains recoverable, and ambiguous state is shown as conflict. Include crash recovery at each new journal transition.

### A2. P1: a mutating check can certify the wrong source revision

`src/sandbox.rs:178` records source before execution; `src/sandbox.rs:440` validates the resulting report and assertion bytes but does not check whether execution changed existing source in the scratch copy. `src/project_services.rs:341` compares the saved pre-run revision with the unchanged working project and reaches `passed on current files` at line 374.

Repro: original `answer.py` returns 0. A pinned external assertion invokes project `prepare.py`, which modifies only the disposable answer.py to return 42, then executes an actual independent `assert answer()==42` and writes a valid JSON report. The original file still returns 0, yet `task verify` exits 0 with both `complete=true` and `behavioral_acceptance=true`. Receipt: `mutating-source.json`. This does not involve a fake report, edited assertion, model, or malicious process; a repair/build/format step can cause the same semantic mismatch.

Work order: bind results to what was actually executed. Capture before/after manifests for existing input source; explicitly distinguish permitted generated outputs from mutations to input source. Return a source-mutated/stale result or expose a reviewable patch to apply and rerun. Do not ban Full access commands. Acceptance: original source changed in scratch cannot be reported behaviorally accepted on the original revision; normal build artifacts in declared output locations remain supported; immutable assertions and unchanged source still pass.

### A3. P2: a completed check loses its evidence if the project lock is briefly busy

`src/sandbox.rs:493` reacquires the project lock once after the subprocess and collection have finished; `Project::open` uses immediate try-lock (`src/project.rs:184`). Failure returns before either emitting or saving the result. Structured report persistence has the same failure point at `src/sandbox.rs:463`; Full terminal results use it at line 671. The existing retry worker in `src/project_worker.rs:17` is not used here.

Repro: configure a short successful check, let it start, acquire its project.lock from a second process, and release the check to finish. CLI exits 1 with only busy error and empty stdout; the next status says `No checks recorded`. Receipt: `locked-evidence.json`. The check-success branch of the script ran; it printed `ACTUAL_SUCCESS_EVIDENCE`, which is not retained by Alt.

Work order: durably spool an execution receipt before trying to attach it to the project database, then wait/retry or reconcile on reopening. Do not rerun a command because persistence was busy. Acceptance: held lock, lock owner termination, cancellation during save, and disk-full conditions preserve the original result and expose an actionable unsaved-evidence state; later recovery records it exactly once.

### A4. P2: execution environment changes can leave checks falsely current

Full checks inherit arbitrary environment variables (`src/sandbox.rs:358`), while freshness fingerprints only seven variables (`src/project_services.rs:94`). The status comparison at line 347 consequently ignores common feature flags or test-selection variables.

Repro: check asserts `APP_FEATURE_MODE == enabled`. It passes with enabled. Invoke `task verify` with disabled: it still exits 0 and marks current files passed. Immediately rerunning the same check with disabled exits 1. Receipt: `environment.json`.

Work order: explicit check execution environment and dependency inputs, with hashes rather than raw sensitive values. Where the complete inherited environment cannot be established, qualify the freshness claim. Acceptance: declared custom variable addition/removal/value change, interpreter and dependency change, and external input changes invalidate old evidence; no raw credentials in evidence or UI.

### A5. P2: apply and undo silently overwrite an external chmod

`src/project.rs:946` hashes bytes only; `src/project.rs:965` always restores the proposal's recorded mode. Change an existing file's mode from 0644 to 0600 after preparing the edit: apply succeeds and returns it to 0644. Change mode to 0640 before Undo: Undo succeeds and returns it to 0644. Receipt: `permissions-checkpoint-repro.json`.

Work order: include relevant mode/identity metadata in conflict detection and preserve or explicitly reconcile external changes. Acceptance: chmod after preparation, chmod before undo, executable scripts, created files, and deletion/recreation; no silent permission widening.

### A6. P2: CLI cannot configure a check containing {report} through the advertised two steps

`src/main.rs:1818` always constructs configure-check with an empty default contract; `src/verification.rs:109` rejects a {report} argument when no report contract exists. `task contract` requires a preexisting check (`src/main.rs:1719`). Reconfiguring an existing command resets its contract, so configure-then-contract does not solve it for placeholder arguments.

Repro: configure-check json-test -- python3 check.py '{report}' -> `Choose a report path first`; contract json-test ... -> `Configure the command first`. Receipt: `placeholder-setup.json`. A script reading ALT_CHECK_REPORT is a workaround, but ordinary CLI users cannot use the supported placeholder flow.

Work order: configure argv and contract atomically or preserve/edit the existing contract while changing argv; show copyable CLI and equivalent TUI paths. Acceptance: one fresh check with {report}, one with {assertion}, updating argv without erasing the contract, and invalid placeholder validation.

## Additional code-confirmed gaps / unproven risks (not reproduced bugs above)

- Check/terminal raw output after 128 KiB per stream is discarded (`src/sandbox.rs:150`), not merely shortened in the TUI. Test counters continue, so late diagnostics can disappear despite known failure. Implement bounded rotating/spooled raw logs and tail/range retrieval. Validate a real failing diagnostic after a large progress stream. This is distinct from the already tested zero-test counter issue.
- State backup enumerates files and snapshots individual SQLite databases sequentially (`src/storage.rs:69`, `90`), with no operation-level consistent point spanning project DB, report files, sessions and configuration. Individual DB consistency is present; cross-file consistency under active changes is unproven. Add restore acceptance under ongoing checkpoints/checks and a backup barrier or manifest generation protocol.
- Snapshot scanning is not atomic against external editors. One scan can observe a mixture of versions while a build/editor writes several files; file count/size bounds also differ between repository browsing and verification. Define supported scope and verify a stable manifest around snapshot materialization rather than claiming a generic atomic repository snapshot.

## Reproduction

CLI suite:

```sh
python3 reproduce_cli.py --alt /absolute/path/to/alt --output /tmp/alt-audit-new-directory
```

It requires Python 3, Linux flock, and a new output directory. No model or engine is needed. This directory contains a successfully rerun receipt for every CLI finding. The checked binary's complete build identity is retained in `build-info.json`. Receipt paths were adjusted when this report was copied into the repository; measured receipts are unchanged.

Checkpoint harness, from this checkout (adjust file paths if copied):

```sh
export CARGO_HOME=/workspace/.cargo RUSTUP_HOME=/workspace/.rustup
export PATH=/workspace/.cargo/bin:/workspace/.alt-tools:$PATH
rustc --edition=2024 docs/evidence/final-audit-2026-10-08/correctness/checkpoint.rs --extern alt_cli=target/debug/deps/libalt_cli-c03f59b2d4f718c4.rlib -L dependency=target/debug/deps -o /tmp/checkpoint-audit
cc -shared -fPIC -O2 docs/evidence/final-audit-2026-10-08/correctness/concurrent_edit.c -ldl -o /tmp/concurrent-edit-audit.so
/tmp/checkpoint-audit /tmp/alt-audit-mode-new permissions
ALT_AUDIT_TARGET=/tmp/alt-audit-race-new/project/answer.py LD_PRELOAD=/tmp/concurrent-edit-audit.so /tmp/checkpoint-audit /tmp/alt-audit-race-new race
```

The rlib is an existing local build product and its hash suffix is build-specific. Rebuild the current checkout if unavailable. Both scratch project roots should be new. The fault interposer touches only the path in ALT_AUDIT_TARGET; it injects once when the application syncs a `.alt-write-*` temporary file.
