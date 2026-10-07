# Small-model work-order delivery

October 7, 2026. This records execution of H01–14 from the
[combined plan](SMALL_MODEL_IMPLEMENTATION_PLAN.md), covering both research passes.
The user target remains Linux, 16 GB RAM, GTX 1070 8 GB VRAM and user-selected
7B/9B Q4 models. [Usage](SMALL_MODEL_USAGE.md) describes the new CLI/TUI workflows.

The cloud-side interfaces and offline training prerequisites are implemented.
**No model weights were trained, no quality preset was promoted, and this cloud
did not qualify a GTX 1070.** Useful SFT needs a reviewed corpus and qualified
training hardware. Distillation/RL depends on a useful SFT baseline. Optional
research adapters need demonstrated benefits before ordinary use.

## Work orders and remaining acceptance gates

| Order | Delivered and exercised | Still required before a quality claim |
|---|---|---|
| H01 | Independent v5 development oracles; four new sealed families; immutable blocked campaign schedules; all attempts/raw requests/source/identity retained. Broken seeds, reference repairs, weak counterexamples, early exit, changed oracle and stale source checked. Historical v4 oracle files remain unchanged. | Complete repeated screening/confirmation and broader real repositories. A bounded pilot is not the planned 96/480-attempt study. |
| H02 | Migrated explicit output/thinking/action/sampling settings; actual-request relay; max_tokens/max_completion_tokens compatibility; full owned template/tokenizer accounting when available; conservative interrupted-call costs and hard shared generation/request caps. A shrinking output allowance also reduces thinking to retain action headroom; the exact allocated body is measured before forwarding. Labelled TUI controls. | Exact provider/architecture feature support and physical memory/latency at each context. External tokenizer accounting remains unknown. |
| H03 | Revision-bound range/symbol edits, exact fallback, concurrent-edit rejection, CRLF/Unicode/duplicate symbols, checkpoint undo and rollback of earlier multi-file writes after a failed batch. Clipped reads do not offer handles for unseen text. | Matched model edit-error and independent-success measurements. Rollback can report conflicts; Full terminal effects are not journal transactions. |
| H04 | Explicit host workflow; persisted goal/plan/decision/hypothesis; current diagnostics and evidence pages. No stale check or prose becomes independent success. | Matched workflow comparisons and novice human journeys. Diagnostic previews do not invent missing expected/actual values. |
| H05 | Six bounded versioned skills; host metadata selection and one active procedure; native/CLI/TUI access to declared executable helpers. Actual broken/fixed Python, Rust, JS, configuration and browser fixtures; real pip/npm advisory and Semgrep skill checks. | No-skill/short-skill/helper model comparisons at equal prompt/effort. Optional browser/audit software remains a separately installed prerequisite. |
| H06 | Current source outlines/import candidates beside lexical retrieval; edited/deleted source invalidation; incremental indexing bytes/time/process-RSS measurements. Metadata browsing avoids forced whole-project snapshots. | Matched localization/recall gains; RSS is a sample of the whole Alt process, not peak or index-only allocation. |
| H07 | Bounded SSE reconstruction; malformed/incomplete/length-limited call rejection; actual host round trip; exact 7B and 9B Q4 CPU probes, including failures. MiMo 9B passed challenge v2, consuming an unpredictable host result. A fixture proves that repeating the input fails. | More artifacts/settings, adversarial schema/cancellation trials and coding quality. Native success is not broad reasoning competence or GPU compatibility. |
| H08 | Serial source copies; Quick/Careful/Thorough; shared time/token/request budgets; current independent selection; early-stop/no-progress policy; preserved interrupted reservations; explicit diff review/apply/conflict/undo. Real Goose with a scripted provider exercised source protection, costs and selection. | Repeated one/two/three-candidate comparisons under equal total allowance. Mock protocol success is not best-of-k model quality. |
| H09 | Explicit offline optimizer driver; immutable body/provenance registry; fresh invocation draft trials; separate actual v5 validation evidence; reviewed activation/diff/history/rollback in CLI/TUI. An actual MiMo 9B optimizer attempt returned malformed tool markup; the draft validator rejects it and raw evidence is retained. | A valid proposal, separate validation run, reviewer and measured gain before any promoted supplement. No hosted reflector is implicit. |
| H10 | Successful v5 trajectory capture retaining native calls/results and raw provider evidence; pending privacy/rights/correctness/split review; native schema validation, family separation and masked assistant targets. | A diverse approved corpus with confirmed source/output rights. No historical “pass” is automatically training data. |
| H11 | Exact dataset/config/checkpoint resume validation; existing optional NF4 QLoRA driver; exact-parent high-precision merge and pinned F16/Q4 export plan/runner with manifests and failure receipts. | Actual supported GPU SFT, merge/export, architecture-specific loaders and unchanged/adapted/Q4 held-out comparisons. No completed adapter exists here. |
| H12 | Offline verified reward scorer: invalid gold, stale source, altered tests, no-op repairs, incomplete output and unsupported claims are excluded; efficiency bonus follows correctness. | Working SFT, permitted teacher/data and qualified GPU resources before SOD/OPD/RL jobs. The scorer is not an RL trainer. |
| H13 | Explicit bounded same-model recursive analysis with lexical span selection, Unicode-safe byte limits, recursive-level reservations, call/time/token caps, source hashes, omitted-input accounting and unverified outputs. Source observations and span hashes persist before synthesis; a stalled second call retains partial progress and its reserved cost. | Reviewed independent retrieval failure, equal-effort retrieval comparison and measured usefulness. No infinite-context claim. |
| H14 | Optional adapter execution contract with pinned executable/scripts/model roles, fresh actual source states, immutable external oracle and captured resource evidence. Deterministic tests reject prose scores, altered oracles and malformed resource declarations. | Individually reviewed OptiLLM/ThinkBooster/PRM/search integrations and measured source behavior/resource visibility. None is installed or promoted by this runner; latent architectures remain separate training research. |

## Live CPU qualification

All live models were explicitly publisher-labelled uncensored/abliterated
derivatives. Probe reports retain runtime, template/tool-schema hashes and actual
requests/responses. Challenge v2 validates a native call, then generates a fresh
host value that was absent from the initial request. The model must consume that
tool result. The earlier predictable echo probe only established a basic round
trip; its legacy `host_result_used` flag does not prove result consumption.
Neither version tests arbitrary software repairs.

| Exact Q4 artifact | Observed result |
|---|---|
| Spark-X2.5-1.7B-Abliterated, darioooooo0o GGUF revision `0193e8f9b55c7f8e9c58799694696a17d8f6c6c5` | Automatic native probe failed: the response produced text/XML rather than a native call. |
| Josiefied-Qwen2.5-7B-Instruct-abliterated, QuantFactory revision `21401ad3fb6d1c6de9d6e078b0d83b7c7a792caf` | 240-second initial probe timed out. A 512-token/temperature-zero probe produced repeated calls until its output limit; a separate required-tool/serial-call 256-token probe also ended at the limit. No native round-trip pass. |
| MiMo-V2.6-Distill-Qwen-9B-heretic, mradermacher revision `4e642ce6d23ee0130bdf57cfe64d644008c042ee` | Challenge v2 passed on CPU at 4096 context, 512 output tokens, 128 reasoning tokens, temperature zero and two runtime threads. It returned the fresh host value after its native call. One measured round trip, approximately 41 seconds including startup. The earlier predictable echo also passed and is retained separately. |

Artifact identities live in `scripts/evaluation_models.py`; the retained
[evidence index](research/2026-10-07-harness-delivery/README.md) links the exact
reports and raw exchanges. The 7B GGUF probe is a different derivative from the
7B HF tokenizer handoff; their ancestry/qualification must not be conflated.
MiMo's inference success does not repair its documented training-loader gap in
the pinned Transformers 4.57.1 starter.

The small-model host-workflow pilot freezes the same artifact, runtime, engine,
8K context, output allowance, sampling, access and shared effort within each
model. Only model-plan versus host-plan changes. Spark and Qwen each passed **0/8**
independent checks. A separate matched Spark pilot compares server-default
thinking against a 128-token thinking budget, keeping the host workflow and total
allowance unchanged; it passed **0/4**. Different completion styles and costs did
not produce correct source behavior. A final MiMo 9B comparison on one repair
family, with/without the Python skill, passed **0/2** under equal 8K context and
five-minute CPU deadlines. It made native reading calls but timed out before a
verified repair. All 22 failures are retained, and no quality preset is promoted.
The subsequent [repair work orders](REPAIR_WORK_ORDERS.md) and
[repair evidence](research/2026-10-07-repair-results/README.md) add compact context,
short required handles, explicit scalar/line-array formats, refreshed source,
parser/definition feedback and truthful synthetic provider errors. A separate
ten-minute matched MiMo pilot passed **3/4 with each interface**; compact reduced
initial input and its passing attempts were faster. The later portable build
passed six of nine MiMo attempts, including an actual 80×24 TUI repair and two
of four held-out families; Rust and the remaining two held-out tasks failed.
The current source then passed a separately planned two-turn MiMo TUI trial,
retaining the complete original request on the follow-up. A failed earlier
continuation attempt and the other checkpoint/settings failures remain recorded.
Two successful development trajectories are
retained as pending training review, not an approved corpus. One repeat on the earlier families cannot establish
statistical gains or nominate a default. The older v0.6 counts (7/120 Spark and
16/120 Qwen) used different
settings and weaker oracles, so these new runs are not a like-for-like improvement
claim over those totals.

The follow-up application checks passed 119 Rust tests, strict Clippy/formatting,
both Goose adapters with both compact formats, 80×24 permission/edit/check/undo
journeys, ten stream-recovery scenarios and the independent-oracle collector.
Fresh language, held-out and live TUI outcomes are recorded separately in the
repair evidence. Native transport, source behavior and normal turn completion
are scored separately.

## Reproduce cloud checks

`bash scripts/setup-cloud.sh` installs the pinned Rust/Goose/dev dependencies and
now exercises the new oracle, training boundaries, adapter contract, helpers,
analysis controller, candidates and TUI alongside existing regressions. It does
not download evaluation weights, install optional GPU/browser/audit stacks or
perform SFT. Browser acceptance here used installed Chromium/Playwright separately.

Research inputs remain in the two
[source](research/2026-10-07-small-model-sources.json)
[ledgers](research/2026-10-07-reasoning-sources.json).
Their published gains remain hypotheses for Alt. The delivery evidence separates
application/protocol fixtures, actual helper subprocesses, live-model behavior
and outstanding GPU/PC/training/quality gates.
