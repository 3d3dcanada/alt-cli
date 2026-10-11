# Final-pass implementation work orders

Baseline and evidence: [final audit](FINAL_AUDIT_2026-10-08.md). These are planned
work orders, not implemented fixes. User-facing dates follow the 2026-10-08 client
context. The target remains Linux, 16 GB RAM, GTX 1070 8 GB VRAM and user-selected
7B/9B Q4 uncensored/abliterated models, with CPU and newer hardware supported.

## Outcomes and release gates

The first goal is reliable behavior under ordinary user interactions. Capability
improvements must then be measured at a fixed model, context and effort budget.
Targets below are proposed acceptance criteria, not measured achievements:

- Zero lost user drafts, profile fields, recorded results or unrecoverable
  intervening edits in the declared failure-injection suite.
- Zero false “behaviorally accepted on current files” results in source,
  environment, assertion, concurrency and report-mutation negative controls.
- All critical setup/recovery actions usable by keyboard at supported sizes.
- At least four of five novice participants complete connection, practice repair,
  independent checking and undo without external coaching; record all assistance.
- A matched model-quality target of at least **15 percentage points** more
  independently successful held-out tasks at the same budget. Report uncertainty
  and per-model results; a small sample or one exercise cannot establish this.
- A separate efficiency target of at least **25% fewer unnecessary prompt tokens
  or repair round trips**, with no lost requirements or correctness regression.
- At least **90% less installed historical-documentation payload**, while keeping
  every original evidence file available in a separate verified archive.

Do not claim “10× smarter” from these targets. Track time to a verified useful
result, interventions, failed/incorrect completions, RAM/VRAM and total effort.

## F01 — Make verification describe the executed source

**Priority:** release blocker. **Findings:** A01, A08, A09, A20, A23.

Capture the input-source manifest before and after check execution. Separate
declared generated outputs from changes to existing inputs. If preparation changes
input source, retain the resulting patch and report a changed-source result;
apply/recheck is an explicit operation. Preserve Full host commands.
Pre/post equality alone does not establish immutable execution: a check can
mutate, test and restore. Define a stronger isolated immutable-input path where
supported, or qualify the claim to observed source snapshots. Keep unrestricted
Full commands available without implying stronger guarantees than they provide.

Make argv, evidence contract and declared execution inputs one atomic check
configuration. Fingerprint effective inputs without persisting raw credentials.
Qualify freshness when an inherited input cannot be accounted for. Spool raw
output and an execution receipt before acquiring the project database lock;
reconcile exactly once after contention or restart. Support bounded raw-log
storage and tail/range access instead of discarding late failures.
Reserve receipt capacity before execution. If storage cannot accept a durable
receipt, expose the unsaved state and available export/recovery path; never claim
that a result was saved or rerun the command automatically.

**Acceptance:** the pinned-assertion mutation repro cannot certify the original;
include a mutate → test → restore negative control for source-identity claims;
normal build outputs still work; environment changes stale evidence; `{report}`
and `{assertion}` work in a newly configured CLI check; lock contention and
interrupted saves recover the original spooled receipt without reexecuting commands;
disk-full injection exercises reserved capacity and explicit unsaved-state recovery;
a failure after a large progress stream remains retrievable.

## F02 — Preserve displaced edits and relevant file metadata

**Priority:** release blocker. **Findings:** A02, A28, snapshot portion of A27.

Define practical conflict guarantees and journal transitions for apply, delete,
undo and batch undo. Retain observed displaced versions and relevant mode/file
identity metadata; expose ambiguous transitions as recoverable conflicts. Review
coordination when two Alt state roots address the same project. Stabilize source
manifests around snapshot materialization or clearly label a changed capture.

**Acceptance:** deterministic external in-place writes and rename-based saves at
each transition, chmod changes, deletion/recreation and crash recovery preserve
recoverable versions and never silently widen permissions. Existing dirty Git
files remain intact. Do not promise a generic filesystem CAS against every
non-cooperating writer; document the supported guarantee precisely.

## F03 — Preserve profiles and every unsent message

**Priority:** release blocker. **Findings:** A03, A04.

Use typed profile patches so a connection edit changes only selected fields.
Store the submitted message separately from the next-message composer. Persist
per-session/project drafts with debounced durable writes and explicit discard or
restore when creating a new conversation. Keep source message IDs through
connection success, failure and retry.
Define and expose the draft durability boundary; drain pending writes on orderly
exit and measure the maximum uncommitted keystroke window on abrupt termination.

**Acceptance:** the customized 4K profile retains every allocation and sampling
field after a name/endpoint/key edit. Text entered during startup survives success,
failure, cancellation and reconnect exactly once. Quit and New preserve the full
draft; abrupt interruption restores the last durable draft within the declared
loss window. Test actual PTY interaction and restart.

## F04 — Maintain an evolving task contract

**Priority:** next capability foundation. **Finding:** A05. **Depends on:** F03.

Retain exact user requirements with message IDs, scope and explicit supersession.
Build a compact active-requirement view that includes middle-turn corrections,
not only the original task. Distinguish user decisions, observed facts and model
hypotheses. Let the user inspect “What Alt currently understands” and correct it.
Retire a requirement only through an explicit user correction or UI decision,
retaining exact source text; a model summary cannot silently supersede it.
When a complete requirement set exceeds the budget, expose the limitation and
offer explicit task decomposition rather than silently losing requirements.

**Acceptance:** unique constraints introduced across 3, 12 and 50 turns survive
restart, cancellation, correction and context pressure. Superseded constraints
are retired with a recorded reason; unrelated tasks do not inherit stale goals.
Include negative checks proving the middle-turn requirement is actually present
in the provider request, not merely stored in the database.

## F05 — Supply relevant source and precise failure packets

**Priority:** next capability foundation. **Findings:** A06, A07, A22.
**Depends on:** F01 and F04.

Merge explicit paths, target symbols, changed files, imports and valid diagnostic
locations; rank and deduplicate within the token budget. A prohibited-file mention
must not crowd out the implementation. Parse structured compiler output where
available and retain diagnostic crate/root, source revision and provenance.
Collect actual named cases, observed counterexample inputs, expected/actual values
and a reproduction command. Include both stdout and stderr in instruction trials.

**Acceptance:** the Cargo-only replay includes the median implementation; a
declared navigation set locates the known target implementation in the top three
results for at least 90% of queries. Oracle-crate paths cannot become implementation handles
without a valid mapping. Repeated spans appear once. Every reported counterexample
is backed by an observed check; independent acceptance remains unchanged.

## F06 — Match edit operations to intent and support executable reasoning

**Priority:** measured capability experiment. **Findings:** A07, A22, A26.
**Depends on:** F01, F04, F05 and the F13 baseline.

Offer concise operations for replacing a symbol/expression, applying a targeted
patch, inserting, creating or deleting. Show exact target scope and compute a
parse/diff preview before applying changes that unexpectedly remove surrounding
definitions. Provide explicit intentional rewrite choices and retain Full
terminal capability. Reuse a small set of clear tools rather than expanding the
default schema with dozens of overlapping tools.
Distinguish an unsupported parser or uncertain syntax from a confirmed error;
support intentional deletion, rename and incomplete intermediate edits explicitly.

Add a short hypothesis → prediction → disposable probe/check → evidence update
workflow. Useful helpers include calculation/code probes, local API references,
compiler diagnostics and test generation as proposed checks. Model explanations
remain hypotheses until execution supports them. Improve serial candidate search
with actual failure context, distinct declared strategies and an explicit check/
recovery reserve within the same shared budget.

**Acceptance:** replay the two observed destructive Rust span edits and explain
the mismatch before mutation, while preserving legitimate explicit rewrites and
unsupported-language workflows. Compare ordinary repair, executable hypotheses and
best-of-k selection at equal total effort. Measure independently verified success,
syntax-breaking edits, repeated failures, requests and time; longer reasoning text
is not a success metric. Promote no default without a measured benefit.

## F07 — Give processes, runtimes and policies one owner

**Priority:** release blocker. **Findings:** A10, A11, A14, A15, A24.

Centralize owned-runtime transitions for chat, benchmarks, qualification, probes
and candidate work. Await shutdown/reuse decisions before starting a replacement.
An explicitly chosen executable is authoritative; missing or incompatible paths
get a direct recovery action. Keep automatic discovery an explicit selection.

Share effective access-policy checks between CLI and TUI at extension entry
points. Retain process-group ownership after the leader exits and use bounded
TERM/KILL/reap transitions. Scope per-launch credentials or supported private
sockets to the owned inference process, including health/tokenization requests.

**Acceptance:** no extra owned model process during idle-connected benchmark or
qualification; missing selected runtime launches no fallback; correct Full access
still executes arbitrary selected tools. Review/Guided extension probes launch no
process or HTTP request. All bounded MCP helpers are gone after failure/cancel/
close. A competing listener cannot impersonate owned-runtime readiness.

## F08 — Complete model import and provider parity

**Priority:** before promoting model compatibility. **Findings:** A12, A13.

Parse bounded GGUF metadata, discover complete numbered shard sets, verify every
member and record aggregate size plus architecture/context metadata. Keep imported
originals in place. Complete authenticated Ollama across inventory, initialization,
streaming native tools, cancellation and reconnect. Build a provider capability
record from observations; label unsupported or unknown tokenization/context fields.

**Acceptance:** truncated metadata, missing/mismatched/changed shards and malformed
counts fail before loading. Valid single/split files remain supported. One exact
authenticated configuration passes the entire adapter journey; incorrect credentials
fail consistently. Test actual Ollama/LM Studio versions when available and retain
protocol-fixture results as a separate category.

## F09 — Make the beginner journey complete by keyboard

**Priority:** release usability. **Findings:** A18, A19, A21; related A04/A20.
**Depends on:** F03 and F07.

Use one action registry for buttons, shortcuts, menus and command-palette entries.
Add unambiguous manual model-ID entry and keyboard allowance/reconnect recovery.
Make network operations cancellation-aware; suppress results from canceled or
superseded operations. Use responsive/scrollable forms with visible focus and
explicit oversized-paste feedback plus file-reference alternatives.

Turn Home into a resumable next-step guide: choose project, choose/connect model,
prove the connection, prepare an actual check, send a first request, inspect result
and undo. Advanced tools remain reachable. Explain missing dependencies with a
direct action that preserves the draft and selected configuration.

**Acceptance:** keyboard-only setup → repair → exhausted allowance → reconnect →
undo at 80×24 and 60×18; all critical dialogs survive resize/long paths/Unicode.
Canceled inventory returns control within one second and never reopens a dialog.
Run five uncoached novice sessions and report completion, time and interventions.

## F10 — Fit 7B/9B models to the actual computer

**Priority:** performance and accessibility. **Depends on:** F07 and F08.

Use exact model metadata, available RAM/VRAM, context, KV type and offload to
propose a small set of candidate configurations. Make estimates visibly distinct
from measurements. Run a short explicit calibration that records loading time,
time to first token, prompt/decode rates, peak memory and actual offloaded layers.
Keep CPU operation, Pascal-compatible CUDA 12 and newer backends available.

Support cancellable configurable loading deadlines and controlled warm runtime
reuse. Preserve artifact-integrity guarantees; a changed model/settings identity
forces appropriate verification and reload. Proposals require an explicit choice;
never silently switch model, quantization, runtime or inference allowance.

**Acceptance:** low-memory/slow-start/unsupported-kernel fixtures fail clearly;
cold and warm results are reported separately. The real hardware matrix covers
GTX 1070 sm_61, an older CPU-only computer and a newer GPU with the same 7B/9B Q4
tasks at 4K/8K. Physical results remain a PC handoff until actually measured.

## F11 — Make long-lived state and installation recoverable

**Priority:** release blocker. **Findings:** A16, A17, A27.

Separate indispensable recovery state from archival inference payloads. Add
inference-session retention/export, storage budgets, a reclaimable-space preview
and supported remediation before backup limits. Preserve active sessions and
integrity/cost receipts. Use streaming backup with a defined consistent generation
across databases, reports and configuration; validate restoration during activity.

Stage and verify a complete installation generation before activation. Preserve a
coherent prior binary/documentation/manifest generation, and state precisely which
generation is active after any interruption.

**Acceptance:** a persistent-state run with thousands of sessions can retain,
backup, upgrade and restore without manual deletion. Inject disk-full, permission,
destination-conflict and interruption failures throughout installation. Each result
has a complete usable generation or explicit recoverable state; inference evidence
retention cannot remove active work or corrupt a check's provenance.

## F12 — Ship a smaller package and an intelligible evidence catalog

**Priority:** delivery efficiency. **Finding:** A25; evidence-collection gap.

Ship concise offline user help, required licenses, manifests and the PC recorder.
Publish historical research/evidence separately with checksums, an index and
links from the app documentation. Preserve all old failures and source identities.
Give each CI stage a versioned receipt and automatic failure-log retention, even
when setup fails before `dist` exists.

**Acceptance:** reduce installed historical documentation by at least 90%; first
run and recovery help remain available offline. Every baseline archive file is
accounted for in the separate evidence manifest. Inject setup/package/preflight
failures and obtain an actionable commit-indexed receipt automatically. Measure
installation time and disk footprint on constrained hardware.

## F13 — Establish a real coding-quality baseline and longevity gate

**Priority:** start the baseline before capability changes. **Finding:** A26/A27.

Declare development, validation and final held-out task families. Include repair,
feature work, repository navigation, numeric boundaries, configuration, multi-file
changes, middle-turn corrections and dirty workspaces. Freeze exact uncensored
7B/9B artifacts, runtime, sampler, tools, native context and total effort. Use at
least 30 held-out families and three repetitions per family/model, with a complete
attempt manifest. Report uncertainty by task family; repetitions are not new
independent problems. This is a planned, budgeted campaign, not a quick smoke test.
Use small shards for iteration; publish all intended, failed and unperformed runs.
Establish the development/validation baseline first. Freeze candidate code and
presets before inspecting final held-out results, including the final baseline;
retain all attempts without using that final comparison for further tuning.

Compare baseline and candidate under matched budgets and shared checkpoints.
Report verified task success, unsupported completion claims, native-tool failures,
interventions, latency, prompt/output cost and peak resources. Add one persistent
workspace longevity run covering indexing, many conversations/projects, external
edits, cancellation, retention, backup, migration and update.

**Acceptance:** aggregate counts derive from immutable per-attempt records; no
selective successful-subset promotion. Final held-out results do not select prompts,
training data or adapters. Software release readiness and qualified model presets
have separate badges. The target lift is 15 percentage points at matched effort;
publish uncertainty and regressions even when the target is missed.

## F14 — Train only after the harness and corpus are qualified

**Priority:** later optimization. **Depends on:** F01–F08 and F13.

Capture real successful repairs/recoveries with rights/privacy/correctness review,
family separation and complete failed-attempt accounting. Meet at least the current
32-training/8-validation-family floor before starting useful SFT. Qualify the exact
full-precision architecture loader and supported GPU path; inference Q4 GGUF files
are not training checkpoints.

Compare the untrained and trained parent model first, then evaluate the exported
Q4 artifact separately. Include instruction-following and uncensored-model behavior
regressions alongside coding quality. Distillation or preference optimization is
an optional experiment after data suitability and baseline evaluation are known.

**Acceptance:** reproducible dataset manifest, reviewed family splits, actual
training/resume/export receipts and a sealed independent comparison. A completed
training job alone does not qualify an improvement. Current status remains no
trained weights and no promoted quality preset.

## Delivery order

| Increment | Orders | Exit condition |
| --- | --- | --- |
| Baseline capture | F13 initial manifest, existing negative repros | Current defects and exact model conditions are frozen before changing behavior |
| Trust and durability | F01–F03, F07, F11 | All reproduced preservation, wrong-source, ownership and update defects have meaningful passing regressions |
| Model assistance | F04–F06 | Matched task results show the effect of requirements, retrieval, diagnostics and edit contracts |
| Beginner/older-PC finish | F08–F10, F12 | Complete keyboard onboarding, measured model fit and smaller verified package |
| Qualification and training | F13 final comparisons, then F14 if prerequisites pass | Honest per-model gains, long-lived-state evidence and no unmeasured promotions |

Implement shared profile patches, action dispatch, process ownership and evidence
commit incrementally behind existing interfaces. Avoid a wholesale rewrite that
would discard the existing tests and obscure which change improved outcomes.
