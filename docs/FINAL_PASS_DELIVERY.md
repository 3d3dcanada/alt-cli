# Final-pass implementation delivery

The approved [14 work orders](FINAL_PASS_WORK_ORDERS.md) have implementation in
this branch. The [beta 3 audit](FINAL_AUDIT_2026-10-08.md) remains the record of
the original defects. The previously published beta 3 does not contain these
changes. Dates follow the October 8, 2026 client context.

This is a substantial reliability and usability change. It does not establish
that a small model is unlimited, ten times smarter, or qualified on a GTX 1070.
Model quality, hardware fit and novice usability have separate acceptance gates.

## Delivered work

| Work order | Implementation | Evidence and limits |
| --- | --- | --- |
| F01: verification | Pre/post source observations, source patch, effective environment and external-file fingerprints, atomic command/contract configuration, preallocated execution receipts, restart reconciliation and bounded raw streams. | Fault tests cover changed/restored source, late output, contention and interrupted writes. Results explicitly describe observed snapshots; `immutable_inputs` remains false. |
| F02: edit recovery | Cross-state project coordination, staged atomic exchange, inode/mode checks, displaced-version retention, permission-aware batch undo and explicit conflict recovery proposals. | Deterministic in-place/rename/chmod/delete/crash tests. Linux exchange support is required; unsupported filesystems fail without an overwriting fallback. No generic filesystem compare-and-swap guarantee against uncooperative writers. |
| F03: profiles and drafts | Profile patches preserve custom fields. Composer and submitted request have separate durable records; autosave, restart recovery, archive/restore and explicit save errors are visible. | Kill/restart, connection failure, next-message, second-window and disk-failure PTYs. A second writer is refused; drafts are not silently merged. |
| F04: exact requests | Every active user request retains its complete text and sequence. Explicit corrections link the previous request, replacement and reason. Standard and compact context fail visibly if requirements exceed their budget. | Reopen, middle-turn, supersession, export and context-pressure checks; model tools cannot erase user requirements. |
| F05: retrieval and feedback | Ranked source/symbol/path/import retrieval with provenance, bounded diagnostic packets, actual independent failures and raw late-output access. | Focused context/feedback tests and real Goose adapter protocol journeys. Retrieval supports the selected model's context; it does not expand native context. |
| F06: tools and candidates | Typed symbol/text replacement and insertion, preview and scope checks, explicit broad rewrites, executable hypothesis/prediction receipts, distinct candidate strategies and a shared effort budget. | Adversarial edit and candidate fixtures. Different strategies are not proof of independent or correct model reasoning. |
| F07: process ownership | Explicit executable selection, awaited runtime retirement, retained process-group ownership, bounded probes, private owned-runtime socket and authenticated proxy, shared extension-policy checks. | Leader-exit, timeout, overflow, competing-listener, policy and live runtime checks. Detached processes and external servers are outside owned process groups. Full access retains arbitrary host commands and network access. |
| F08: compatibility | Bounded GGUF structure/tensor inspection, complete imported shard sets and aggregate identity, authenticated Ollama throughout, observed provider capability records. | Corrupt/missing shard and adapter fixtures. Unknown tensor layouts disclose incomplete payload verification; actual third-party server releases remain separate qualification. |
| F09: terminal workflows | Shared keyboard actions, manual model IDs, responsive selection/cancellation, raw logs, check dependencies, request correction, recovered-file preview, storage controls and visible loading state. | Actual PTY journeys at 120×40, 80×24 and 60×18; five uncoached novice sessions remain unperformed. |
| F10: runtime fit | Metadata-aware RAM/VRAM estimates, explicit fit selection, loading deadline, streamed timing, sampled memory, cold/warm state and observed offload reporting. | Exact uncensored 7B Q4 loaded and answered on CPU through the owned runtime. GTX 1070/CUDA, physical peak memory and larger context remain unmeasured. |
| F11: state and updates | Shared state-write barriers, consistent backups, active inference leases, archived payload exports, retention budget, atomic installation generations and rollback. | Storage concurrency, backup/restore and interrupted installer tests; historical raw payload archives are explicitly separate from core state backup. |
| F12: distribution | A small offline guide set, complete separately verified research archive bound to the application, early CI-stage receipts and app/research attestation targets. | Installer verifies helpers before executing them. The full documentation index preserves source hashes; historical research is not deleted. |
| F13: qualification | Frozen baseline/candidate/evaluator/model identities, 30 final task families, repeated paired accounting, oracle controls, and persistent state stress. | [Qualification report](FINAL_PASS_QUALIFICATION.md). The complete 360-slot final matrix is unperformed. Pilot cells and deterministic controls do not close the quality gate. |
| F14: training | Reviewed collection ledger, separate training/validation families, original-record binding, exact parent/loader/GPU checks, resume/export and independent Q4 qualification handoff. | [Training guide](../training/README.md). No weights trained: zero approved families and no qualified CUDA loader in this cloud environment. |

## Validation records

The final integrated Rust source run passed **173 tests** with one explicitly
ignored longevity test, which was executed separately. Formatting and strict
all-target Clippy passed. The exact source fingerprint and command logs are in
[software evidence](evidence/final-pass-2026-10-08/software/source-validation.json).
Terminal, adapter, packaged-upgrade and candidate-model stages have their own
receipts and are not included in this unit-test count.

The final frozen debug executable passed **11 terminal gate groups** and **12
headless/provider integration groups**, including real Goose with both adapters,
authenticated Ollama, both compact formats, native tool-result checks, browser
skills, candidate workflows and ten stream-recovery scenarios. Fourteen campaign
controller tests and 55 training-gate tests also passed. The dependency audit found
no denied advisory. Initial test-helper failures and their corrected retries are
retained in the [terminal](evidence/final-pass-2026-10-08/tui/final-debug/final-summary.json)
and [protocol](evidence/final-pass-2026-10-08/protocols/README.md) records.

The clean portable candidate is commit
`16a51866306efd7fb22270181eafca2cf6bcd5f6`, binary SHA-256
`54b0cf02702df83afa96c3c31b5905e2c5311d787b9400f7ac9ac745e0486e51`.
It passed **all eight packaged release gates**, including old/current Linux,
installed TUI, research integrity, SBOM and actual beta 3 migration/rollback.
A separate upgrade/backup-restore/rollback/re-upgrade of the aged workspace kept
all six SQLite databases and the original state digest unchanged. The retained
active state contained 1,001 sessions and 4,004 events after the longevity
archive/restore steps. See [delivery receipts](evidence/final-pass-2026-10-08/delivery-candidate/).

The same portable binary passed 30 soak rounds and 30 keyboard-pressure journeys
(ten each at 120×40, 80×24 and 60×18), with zero surviving owned processes.
Observed input p95 was 48.7 ms and cancellation p95 65.5 ms in those cloud
fixtures. These are observations, not a latency guarantee for inference or older
hardware. The [stress receipts](evidence/final-pass-2026-10-08/portable-stress/)
retain the initial stale-test-navigation failures and corrected rerun.

The measured application archive is **9,836,696 bytes**, versus **24,468,768** for
published beta 3. Its unpacked payload fell from 195.0 MB to 43.7 MB (77.6%).
The separate 15,197,705-byte research archive preserves every one of the 9,436
documentation/evidence files in that package snapshot. Installed historical
documentation fell by more than 99%; the full application archive did not shrink
by 90%. Later evidence files belong to the repository and subsequent package
snapshots; these measured archive hashes are retained unchanged.

The retained [evidence directory](evidence/final-pass-2026-10-08/) separates
deterministic software tests, actual provider protocol journeys, uncensored
model attempts and unperformed work. Stage receipts identify their tested binary
or source stage; a pass on an earlier stage is not represented as a final-binary
measurement. All model attempts, including cancellations and failed repairs,
remain in the ledger.

The persistent workspace run created 2,000 sessions and 8,000 fixture events
across five projects, exercised index cancellation and external edits, archived
1,000 sessions, restored one, exported 32 old inference payloads while preserving
an active lease, and checked a backup/restore with SQLite integrity. This is a
state durability test, not thousands of real model conversations.

The real runtime lifecycle check used the exact Josiefied Qwen2.5 7B abliterated
Q4_K_M artifact, 4K context, two CPU threads and a 24-token response cap. It
verified private loading/generation, rejection of an unauthenticated proxy
request, correct behavior with unusable inherited HTTP proxies, and cleanup.
This is a lifecycle result, not task-quality evidence.

## Use the new workflows

- **Connections:** edit a profile without resetting its inference/runtime fields;
  type a model ID if the server cannot list models.
- **Chat:** an unsent draft survives restart. A submitted request stays separate
  from the next text you type. Review recovered drafts before resending.
- **Task:** inspect exact requirements, explicitly replace a changed requirement,
  inspect raw check output and declare external inputs or generated output paths.
- **Recovered files:** preview an observed displaced version and turn it into a
  normal tracked proposal. Applying it still checks the current file.
- **Hardware and runtime:** inspect estimates, choose settings explicitly, then
  measure the selected runtime/model. Warm reuse never silently changes a model.
- **Storage:** inspect/set retention and export inference data; back up core state
  before update. Installation generations support explicit rollback.

See [CLI](CLI.md), [workspace](WORKSPACE_GUIDE.md), [runtime fit](RUNTIME_FIT.md)
and [installation](INSTALLATION.md) for exact commands and backup scope.

## Gates that remain open

1. Complete the frozen 30-family, three-repeat, two-model, two-arm final campaign
   before claiming the proposed 15-point gain. Its maximum declared model-turn
   budget is 60 inference-hours; all unperformed slots remain visible.
2. Measure prompt/round-trip savings on that comparison before claiming the
   proposed 25% efficiency gain. Cloud CPU pilot timings shared with compilation
   are unsuitable for a causal speed claim.
3. Run the [PC recorder](PC_TESTING.md) on the user's Linux/GTX 1070/16 GB system
   with the exact 7B/9B Q4 files and runtime. Test actual Ollama/LM Studio versions,
   accelerated runtime and the intended context/offload settings.
4. Conduct five uncoached novice sessions with assistance recorded. Automated
   keyboard journeys validate controls, not novice success rates.
5. Collect and independently review at least 32 training and eight validation
   families, qualify the original training parent on suitable CUDA hardware,
   and compare the resulting adapter/Q4 artifact with the frozen baseline.

Keep Full access available for authorized arbitrary host work. Guided isolation
and reviewed proposals remain explicit choices; no model, provider, runtime,
tool set or access mode is silently substituted to make a check pass.
