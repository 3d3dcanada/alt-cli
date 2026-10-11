# Final audit: beta 3 and the next improvement cycle

Review date: 2026-10-08. Baseline: `v0.6.0-beta.3`, commit
`fa0c8e4130ec4d13c59193436679c4e059c22dee`, runtime source digest
`54bb6215b1e87579f3534c347f38019cf569e91386237352e314ba5d025976f1`.

## Assessment

Alt has substantial working infrastructure: a Rust terminal application, real
Goose adapters, local model management, tracked edits, independent assertions,
recovery features, and a verified public release. Its release provenance is
strong. Its current user-intent, verification and lifecycle contracts still have
defects that should be addressed before recommending it for important projects.

The highest-value next release should fix those contracts and remove avoidable
burdens from 7B/9B Q4 models. Increasing context, adding tools or requesting longer
reasoning cannot compensate for a dropped user requirement, an incorrect source
location, or a check result attributed to the wrong source revision.

“Ten times better” is a product ambition, not a result established by this audit.
The [implementation work orders](FINAL_PASS_WORK_ORDERS.md) define measurable
outcomes for reliability, first-task completion, model correctness and cost.

## Scope and method

Six parallel reviews covered user interaction, core correctness, the model
harness, runtimes/adapters, execution boundaries, and delivery/operations. The
review included source, existing tests, retained live-model traces and release
receipts. Bounded CLI and real-terminal reproductions used disposable projects,
state folders and deterministic fixtures. A standalone Rust replay exercised the
current compiled library. No new real-model requests or training were performed.

Sixteen existing compact-context and PC-readiness tests were rerun and passed.
The previously reported 123 Rust tests and complete release CI passed on this
baseline; this audit did not rerun the entire setup. The new reproductions expose
combinations those tests do not cover. Green existing tests and the defects below
can therefore both be true.

Application source and the published archive were not changed during this audit.
The findings remain open. Small receipts, fixture source and terminal captures
are retained in [audit evidence](evidence/final-audit-2026-10-08/README.md).
Weights, databases, compiled probes and large/sparse test files are excluded.

Evidence labels: **reproduced** means a bounded execution demonstrated the issue;
**trace** means it appears in a retained real-model run and is supported by code;
**source** means the implementation establishes the path but its full effect was
not measured; **gap** means a proposed qualification or product improvement.
P1 findings should block a reliability promotion; P2 findings need scheduled
remediation; P3 concerns affect usability or efficiency without the same urgency.

## Findings

### Correctness and preservation

| ID | Priority / evidence | Finding and consequence | Primary source / receipt |
| --- | --- | --- | --- |
| A01 | P1, reproduced | A check can modify the disposable source, pass a real pinned assertion, and mark the unchanged incorrect working source behaviorally accepted. The executed source revision is not rechecked. | [sandbox.rs:178](../src/sandbox.rs#L178), [sandbox.rs:440](../src/sandbox.rs#L440), [project_services.rs:374](../src/project_services.rs#L374); [receipt](evidence/final-audit-2026-10-08/correctness/mutating-source.json) |
| A02 | P1, reproduced | An external editor write between the stale-content check and replacement is lost; Undo restores the older proposal image rather than the intervening user version. | [project.rs:999](../src/project.rs#L999), [project.rs:968](../src/project.rs#L968); [receipt](evidence/final-audit-2026-10-08/correctness/race-checkpoint-repro.json) |
| A03 | P1, reproduced | Saving an otherwise unchanged connection resets custom inference settings and the uncensored claim. The example changed context from 4K to 8K and removed its output, request, token and sampling settings. | [keys.rs:719](../src/tui/keys.rs#L719), [app.rs:1316](../src/tui/app.rs#L1316); [before](evidence/final-audit-2026-10-08/ux/connection-before.json), [after](evidence/final-audit-2026-10-08/ux/connection-after.json) |
| A04 | P1/P2, reproduced | Text typed during initial connection is overwritten by the frozen submitted prompt. New and quit/restart also discard unsent drafts without restoration. | [app.rs:834](../src/tui/app.rs#L834), [app.rs:891](../src/tui/app.rs#L891), [app.rs:1397](../src/tui/app.rs#L1397); [receipt](evidence/final-audit-2026-10-08/ux/draft-during-connect-result.json) |
| A05 | P1, reproduced | In Compact mode a third turn can omit a correction supplied in turn two. The original goal survives, but intervening request notes are excluded and each engine turn starts a new native session. | [project.rs:226](../src/project.rs#L226), [compact_context.rs:163](../src/compact_context.rs#L163), [engine.rs:563](../src/engine.rs#L563); [replay](evidence/final-audit-2026-10-08/harness/replay.txt) |
| A08 | P2, reproduced | A completed check loses its output and saved result if the project lock is briefly occupied while evidence is committed. The CLI returns only a busy error. | [sandbox.rs:493](../src/sandbox.rs#L493), also lines 463 and 671; [receipt](evidence/final-audit-2026-10-08/correctness/locked-evidence.json) |
| A09 | P2, reproduced | An inherited feature flag can change while the old passing result remains marked current. Rerunning immediately with that same changed environment fails. | [project_services.rs:94](../src/project_services.rs#L94), [sandbox.rs:358](../src/sandbox.rs#L358); [receipt](evidence/final-audit-2026-10-08/correctness/environment.json) |
| A20 | P2, reproduced | CLI check configuration has a circular prerequisite for `{report}`; reconfiguring also resets its existing contract. | [main.rs:1719](../src/main.rs#L1719), [main.rs:1822](../src/main.rs#L1822), [verification.rs:109](../src/verification.rs#L109); [receipt](evidence/final-audit-2026-10-08/correctness/placeholder-setup.json) |
| A28 | P2, reproduced | Apply/Undo ignores external permission changes and can widen a file from 0600 back to 0644. | [project.rs:946](../src/project.rs#L946), [project.rs:965](../src/project.rs#L965); [receipt](evidence/final-audit-2026-10-08/correctness/permissions-checkpoint-repro.json) |

A01 is not a fabricated test report. The independent assertion remains pinned,
actually executes the repaired disposable source, and actually passes. The original
file still returns the wrong answer. Results must describe the source that was
executed; legitimate generated build outputs need their own declared handling.
Matching pre/post manifests alone cannot prove immutability during execution;
the fix must also address temporary mutation followed by restoration or qualify
its source-identity claim.

A02 uses deterministic fault injection to schedule a valid external-write
interleaving. It establishes a possible loss, not its everyday frequency. A fix
must define attainable filesystem guarantees, preserve displaced versions and
surface conflicts. A generic atomic compare-and-swap against every unrelated
editor cannot simply be promised.

### Model reasoning and tool interaction

| ID | Priority / evidence | Finding and consequence | Primary source / receipt |
| --- | --- | --- | --- |
| A06 | P1, reproduced + trace | Any matched filename suppresses lexical retrieval. “Implement median … do not change Cargo.toml” includes Cargo.toml and omits the implementation, even in a two-file project with available context. | [compact_context.rs:220](../src/compact_context.rs#L220); [replay](evidence/final-audit-2026-10-08/harness/replay.txt) |
| A07 | P1/P2, trace | An independent test crate's `src/lib.rs` warning is mapped to the implementation crate. Arbitrary diagnostic line spans become editable handles; the model replaces incomplete function spans and damages the function before receiving syntax feedback. Duplicate spans consume context. | [workflow.rs:253](../src/workflow.rs#L253), [toolbox.rs:830](../src/toolbox.rs#L830), [toolbox.rs:908](../src/toolbox.rs#L908) |
| A22 | P2, trace/source | The Rust failure reports actual/expected values without its concrete input, and the instruction optimizer reads only stderr although the assertion failure is in stdout. Useful failure information is omitted. | [acceptance_projects.py:73](../scripts/acceptance_projects.py#L73), [acceptance_projects.py:126](../scripts/acceptance_projects.py#L126), [optimize_instructions.py:36](../scripts/optimize_instructions.py#L36) |
| A23 | P2, source | Check output beyond 128 KiB per stream is discarded. A late diagnostic can be unavailable even though counters detect failure. | [sandbox.rs:150](../src/sandbox.rs#L150) |
| A26 | P1 qualification gap | Current software CI proves application/protocol behavior; automated model-matrix choices omit the actual target 7B/9B artifacts. Existing real-model cohorts are too small or differently configured to establish general improvement. | [model-matrix.yml:8](../.github/workflows/model-matrix.yml#L8), [compatibility](COMPATIBILITY.md), [training status](../training/status/repair-capture.json) |

The configured MiMo Rust trace is especially informative. Its failure feedback
pointed at the wrong crate; the model replaced a 12-line read span with two lines,
restored the function, then damaged another partial span. It also guessed at the
missing boundary input. These observations support concrete harness changes;
they do not prove those changes will solve the arithmetic problem.

Raw requests are retained under
`docs/evidence/pc-ready/live/mimo9b-rust-configured/8192-rust-feature-1/state/inference/95b5cf5f-4628-4c7b-80a2-4cbf3ed5f5f3/`:

- `8f81af16-2a9a-4fd1-966c-1ad4436a96df-request.json`: wrong-root diagnostic and source packet.
- `ac7e75e6-754e-438e-9ef7-200dc6da86e8-request.json`: first destructive span replacement.
- `f06e1e36-20e7-423b-9bea-40ddfe230691-request.json`: later partial replacement.

The original failed run stays unchanged. Future structured counterexamples must
come from observed checks, without adding a reference implementation or changing
the held-out oracle to make the result pass. Treat any new feedback format as a
new declared harness condition in comparisons.

### Runtimes, extensions and model acquisition

| ID | Priority / evidence | Finding and consequence | Primary source / receipt |
| --- | --- | --- | --- |
| A10 | P1, reproduced | An invalid explicitly selected runtime silently resolves to a different managed/PATH runtime. The same fallback pattern affects configured engine paths. | [runtime.rs:150](../src/runtime.rs#L150), [runtime.rs:163](../src/runtime.rs#L163); [receipt](evidence/final-audit-2026-10-08/runtime/audit-summary.json) |
| A11 | P1, source | TUI benchmarking/qualification can start a second owned model while an idle connected workspace still holds the first, threatening the target's 8 GB VRAM and invalidating measurements. No physical OOM was attempted. | [management.rs:252](../src/tui/management.rs#L252), [management.rs:806](../src/tui/management.rs#L806), [benchmark.rs:39](../src/benchmark.rs#L39), [workspace.rs:267](../src/workspace.rs#L267) |
| A12 | P2, reproduced | Import accepts a four-byte GGUF magic-only file and records no split pieces for a numbered first shard. Imported additional shards lack the downloaded-set integrity coverage. | [models.rs:342](../src/models.rs#L342), [models.rs:354](../src/models.rs#L354), [models.rs:535](../src/models.rs#L535); [receipt](evidence/final-audit-2026-10-08/runtime/audit-summary.json) |
| A13 | P2, reproduced | Authenticated Ollama inventory succeeds, then engine initialization rejects the same configuration, although the relay already attaches bearer authentication. | [engine.rs:182](../src/engine.rs#L182), [inference.rs:581](../src/inference.rs#L581); [receipt](evidence/final-audit-2026-10-08/runtime/ollama-auth-reproduction.json) |
| A14 | P1, reproduced | Headless `extensions check` executes the configured host program even with `--access review-only` or `--access guided`. The TUI enforces the Full-access requirement. | [main.rs:1048](../src/main.rs#L1048), [management.rs:875](../src/tui/management.rs#L875); [receipt](evidence/final-audit-2026-10-08/mcp-policy/report.json) |
| A15 | P2, reproduced | Closing an MCP discovery connection can leave a helper that ignores TERM running after the leader exits. | [extensions.rs:352](../src/extensions.rs#L352), [extensions.rs:104](../src/extensions.rs#L104); [receipt](evidence/final-audit-2026-10-08/mcp-cleanup/report.json) |
| A24 | P2 hardening, source | The owned llama.cpp loopback port has no per-launch credential; readiness trusts an unauthenticated health response after releasing its port reservation. Other local accounts can access the underlying endpoint. | [runtime.rs:360](../src/runtime.rs#L360), [runtime.rs:466](../src/runtime.rs#L466) |

A14 is a CLI policy inconsistency, not a demonstrated model escape. A24 is local
endpoint ownership/accounting hardening, not evidence of remote exposure or
conversation disclosure. Full access should retain ordinary host commands,
networking, installations and user-selected external tools.

### Beginner experience and long-term operation

| ID | Priority / evidence | Finding and consequence | Primary source / receipt |
| --- | --- | --- | --- |
| A16 | P1, reproduced | Inference traces accumulate outside retention coverage. State above 512 MiB cannot be backed up, and package upgrades require that backup. Normal long-lived use lacks a supported cleanup path for its largest evidence bucket. | [inference.rs:577](../src/inference.rs#L577), [storage.rs:43](../src/storage.rs#L43), [storage.rs:227](../src/storage.rs#L227), [install-package.sh:66](../scripts/install-package.sh#L66); [receipt](evidence/final-audit-2026-10-08/delivery/delivery-receipt.json) |
| A17 | P2, reproduced | The installer can exit unsuccessfully after replacing the executable, when later documentation copying fails. The prior binary is preserved, but the installed generation is partial. | [install-package.sh:74](../scripts/install-package.sh#L74), [install-package.sh:98](../scripts/install-package.sh#L98); [receipt](evidence/final-audit-2026-10-08/delivery/delivery-receipt.json) |
| A18 | P2, reproduced/source | Allowance recovery lacks a keyboard/palette route. The manual model-ID shortcut uses Ctrl+M, which ordinary terminals encode as Enter; the PTY reproduction instead runs inventory. | [keys.rs:96](../src/tui/keys.rs#L96), [keys.rs:398](../src/tui/keys.rs#L398), [keys.rs:786](../src/tui/keys.rs#L786); [receipt](evidence/final-audit-2026-10-08/ux/results.json) |
| A19 | P2, reproduced | Canceled connection inventory ignores its cancellation token and later reopens the model dialog. | [keys.rs:711](../src/tui/keys.rs#L711), [app.rs:1231](../src/tui/app.rs#L1231); [receipt](evidence/final-audit-2026-10-08/ux/cancel-inventory-result.json) |
| A21 | P3, reproduced/source | The 60×18 connection form overlaps controls/help; input above 64 KiB is silently ignored. | [view.rs:1497](../src/tui/view.rs#L1497), [input.rs:39](../src/tui/input.rs#L39); [screen](evidence/final-audit-2026-10-08/ux/connection-60x18.txt) |
| A25 | P2 efficiency gap | The desktop package includes 9,129 historical documentation/evidence files totaling 150.85 MB, about 78% of its unpacked payload. | [package-linux.py:47](../scripts/package-linux.py#L47), [install-package.sh:98](../scripts/install-package.sh#L98) |
| A27 | P2 qualification gap | Current stress loops do not establish weeks of one growing workspace. Cross-file backup consistency during active changes and stable multi-file snapshot capture need dedicated qualification. | [soak.py:28](../scripts/soak.py#L28), [storage.rs:69](../src/storage.rs#L69), [sandbox.rs:178](../src/sandbox.rs#L178) |

## What to preserve

Keep the independent check contracts and negative controls, source checkpoints,
explicit access modes, pinned artifact digests, resumable downloads, actual
inference receipts, and honest failed-model records. Keep the exact-tag CI and
attestation gates. Package integrity and model usefulness are distinct measures.

There is no reason to replace Rust or discard the entire implementation. The
most useful structural change is to give shared operations one owner: profile
patching, draft submission, action availability, process ownership, evidence
commit and installation activation. The demonstrated divergence between CLI and
TUI and duplicated profile construction justify that consolidation. File length
alone is not a reason for a rewrite.

## Recommended sequence

1. **Trust and preservation release:** F01–F03, F07 and F11. Fix wrong-source
   verification, lost edits/messages/settings, runtime ownership and update
   durability. Preserve all capabilities of deliberately selected Full access.
2. **Small-model effectiveness release:** F04–F06, supported by the F13 baseline.
   Retain evolving requirements, retrieve the correct source, provide concrete
   diagnostics, and make edit intent precise. Measure verified task success at
   identical inference budgets.
3. **Beginner and older-PC release:** F08–F10 and F12. Complete adapter/import
   paths, keyboard recovery, progressive setup, model-fit measurement, warm
   runtime reuse and a smaller download.
4. **Evidence-led optimization:** F13–F14. Test hypothesis/probe loops and
   cost-aware candidate search, then training when enough reviewed families and
   a supported training environment exist.

The work orders define dependencies and acceptance. Physical GTX 1070/CUDA 12
performance, a true 16 GB desktop memory budget and uncoached novice sessions
remain measurements that this cloud cannot substitute for.
