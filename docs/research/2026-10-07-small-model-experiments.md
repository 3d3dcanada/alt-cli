# Proposed small-model harness experiments

**October 7, 2026. Proposed work, not an implementation or benchmark result.**
Read [the research](2026-10-07-small-model-harness.md) for sources, limitations and
the reason for this order. Existing Alt functionality and the 0.6 evidence remain
the baseline. Preserve arbitrary Full access and the selected model.

## First tranche

| Order | Work order | Concrete deliverable | What would justify keeping it |
|---|---|---|---|
| 1 | SM-01: effective model profile | Record exact template/parser, sampling, reasoning, output/context budgets and actual transmitted request; explain supported settings in the UI | Reproducible native-tool behavior; no silent ignored controls or model substitution |
| 2 | SM-02: simpler edit protocol | Revision-bound range/symbol handles with atomic writes, diff, checkpoint and syntax feedback | Fewer failed edit attempts **and** better independent source correctness under matched budgets |
| 3 | SM-03: curated executable skills | Small versioned skill format, host-side metadata search and 4–6 narrowly scoped initial packs | Skills beat no-skill controls on applicable tasks without hiding failures or flooding context |
| 4 | SM-04: task controller | Explicit inspect/patch/check/recover stages, durable harness-authored workflow state and compact next-action requests | More real edits/checks per model budget, fewer unchanged projects, acceptable latency |
| 5 | SM-05: compact diagnostic interface | Machine-readable failure locations, codes, actual/expected values where available and raw-log references | Lower irrelevant exploration and fewer unsupported explanations; full logs preserved |
| 6 | SM-06: structural retrieval | Budgeted symbol/dependency map plus lexical search and source-revision provenance | Better localization/recall at equal prompt budget, with measured indexing/memory overhead |

SM-01 and SM-02 are the first experiment candidates. SM-03–05 compose naturally,
but measure each separately before attributing gains to the combined system.

### SM-01: model profiles without hidden defaults

Inspect the serving request rather than trusting settings labels. Record the
checkpoint SHA-256, quantization, tokenizer/template hash, runtime/engine versions,
tool format, total output allowance, reasoning allocation, context reservation,
sampling/seed settings and cache types. A backend unable to expose a control must
report that fact. Preserve the exact external endpoint/model selected by the user.

Retain current Spark-off as a baseline. Test bounded reasoning only with sufficient
remaining action tokens and a declared total budget. Qwen Instruct 2507 is
non-thinking; do not invent a reasoning ablation for that architecture. Keep Q4
versus Q6/Q8 as a separate factor from harness changes. Do not claim PC compatibility
from cloud CPU results or a successful small native-tool probe.

### SM-02: editor correctness before model evaluation

Exercise ambiguous names, duplicate lines, changed files, stale handles, CRLF,
Unicode, empty files, final-newline preservation, decorated functions, comments,
unsupported syntax, nested symbols and edits that fail halfway through. Verify
that stale/ambiguous operations do not write, and successful edits can be undone.
Never silently expand an edit's scope or fuzzy-apply it elsewhere.

Compare at least these representations on the same development problems:

- Existing exact-text replacement.
- A small typed range/handle replacement, with freshness checked by the host.
- Symbol-body replacement for supported languages; explicit fallback elsewhere.

Do not start by giving a 1.7B model a large patch language with registers, moves
and implicit matching. An inspected upstream editing implementation is a reference,
not permission to skip component/license review or correctness checks.

### SM-03: usable skills without a giant instruction library

Use compatible `SKILL.md` packaging, but establish tighter Alt-specific initial
budgets. Candidate budgets to measure: approximately 300–700 tokens for the active
procedure and a small bounded metadata shortlist; these are design hypotheses,
not universal optimal limits. Load additional examples/references only when needed.

Each pack supplies metadata, prerequisites, supported versions, procedure,
deterministic helpers, output schema, recovery hints, license/provenance and
independent checks. A tool pack and a skill are distinct: one exposes operations;
the other explains and implements how to combine them for a task.

Start with Python test repair, Rust diagnostics, JavaScript package setup and
local browser-flow checks. Dependency/security and configuration/data procedures
can follow. Reuse existing Alt checks/packs rather than duplicate them.

For each skill compare: no skill; short curated procedure; procedure plus executable
helper. A deliberately irrelevant skill can test context burden in an explicit
negative-control cohort. Record selection mistakes separately from execution
mistakes. A generated skill stays a candidate until reviewed and tested against
new tasks. Skills never change access permissions merely by declaring tools.

### SM-04–05: move housekeeping and observation into Rust

Persist a visible workflow with a current stage, selected checks and source
revision. “Harness saved the workflow” and “model proposed a plan” must remain
different events. If the experiment changes how a required plan is satisfied,
declare that configuration change; keep the existing profile available.

The model gets the immediate goal, relevant source, latest concrete failure and
available operations. Host progress messages explain routine activity to a novice
without spending model tokens on repeated narration. Let the engine/controller
execute configured checks after a candidate edit, preserving separate subprocess
results and user-visible actions. Do not declare completion based on fluent text.

A failed check should provide an observation packet like this **proposed format**:

```json
{
  "stage": "repair",
  "source_revision": "...",
  "check": "median-behavior",
  "status": "failed",
  "location": "src/lib.rs:18",
  "actual": "Some(0.0)",
  "expected": "Some(-0.5)",
  "raw_evidence": "check-42",
  "next_inspection": "median"
}
```

Only populate fields actually present in trusted diagnostics. The host must not
invent the failing input or a root cause. Preserve raw stdout/stderr, truncation
markers, process exit, timeouts and the exact tested revision. Avoid exposing
sealed evaluation answers to the candidate beyond the feedback policy fixed for
that cohort. Use the same feedback policy in both comparison arms.

### SM-06: bounded context, measured retrieval

Start with current lexical retrieval plus symbol outlines and dependency neighbors.
Separate user requirements, observed facts and model hypotheses. Invalidate source
excerpts after edits. Preserve lookup handles to raw evidence outside the prompt.
Test misleading filenames, changed values, deleted symbols, long logs, distracting
history and source-attribution accuracy after restart.

Add embeddings/reranking only after demonstrating a retrieval gap. Record the
extra model and its resource/license implications. A reranker or summarizer is
another inference dependency, not free context capacity.

## Subsequent experiments

| Work order | Experiment | Prerequisite and explicit cost |
|---|---|---|
| SM-07 | Native tool schema/grammar enforcement | Actual provider support; measure parsing separately from semantic correctness |
| SM-08 | Small programmatic tool compositions | Verified recipes first; compare generated code actions on suitable tasks; count execution errors and all subprocess work |
| SM-09 | Sequential alternative repairs selected by tests | Robust independent verifiers, isolated snapshots, declared candidate/time/token budget; preserve every failure |
| SM-10 | Offline DSPy/GEPA prompt/skill optimization | Explicit permitted optimizer model, development-only feedback, separate validation, recorded optimization cost |
| SM-11 | Verified trajectory dataset and optional LoRA/RL | Correctness-reviewed examples, broader checks, data/model licenses and qualified training hardware |
| SM-12 | Large-input recursive analysis | A real task where retrieval is inadequate; capped recursive cost, one-model serial execution first |

Keep optional tools available outside these profiles. Choosing a smaller active
interface should simplify model work without making the product globally incapable
of arbitrary shell, network, testing or user-selected integrations.

## Evaluation that can support an improvement claim

### Repair the measurement boundary first

The 0.6 oracles remain immutable historical evidence. Their inspected “held-out”
problems are now known to us. Add the discovered date, deduplication and isolated-node
counterexamples to a **new versioned development suite**, clarify output contracts
such as `python-config`, and add genuinely new held-out task families that are not
used for prompt/skill tuning. Increasing coverage can lower a score without making
the model worse. Never compare old/new oracle counts as if only the harness changed.

Validate broken seeds, reference repairs, selected incomplete repairs, early exit,
test deletion and stale-source behavior. Full access is not adversarial containment;
an external verifier must evaluate preserved source under its stated assumptions.
Separate failures to collect/run an observation from model task failures, retaining
both. Do not silently discard timeouts or restarts.

### Screen one factor at a time

A proposed inexpensive first screen is **8 development tasks × 3 repetitions ×
2 arms × 2 models = 96 attempts**. It is exploratory and too small to establish a
broad reliability guarantee. Include the observed failure types, but also tasks
where the baseline already works to detect regressions. Keep the same suite for
both arms and avoid selecting only cases favorable to the proposed tool.

A subsequent confirmation could use **20 development tasks × 5 repetitions plus
4 new held-out tasks × 5 repetitions = 120 per arm per model**, or **480 attempts**
for two arms and two models. Freeze the final design before held-out evaluation.
This is a proposed budget, not work executed in this research pass. Larger and
more varied real repositories will still be needed for a product-level claim.

Match exact model/quantization, runtime, parser/template, context, sampling,
reasoning, total generation allowance, execution policy, task state, tools, checks
and wall-time limits except for the factor being tested. Record paired seeds where
supported; they do not guarantee identical sampling across changed prompts or
providers. Randomize/block execution order to reduce host-load/time effects.

For a profile/budget experiment, declare the changed controls and report the
quality–latency/resource tradeoff. Do not label a larger budget an isolated harness
improvement. Candidate retries need a fixed **total** budget, not a fresh unlimited
allowance for each candidate. Compare sequential execution on the target PC;
concurrent cloud inference cannot establish its responsiveness.

### Metrics

Record independent task success, regressions, unsupported completion claims,
source unchanged, first real edit latency, invalid edit/schema/semantic arguments,
turns, generated tokens, prompt size, tool output volume, wall time, cancellation,
user interventions and selected-skill accuracy. Report `pass@1`, selected-best-of-k
and all attempts separately. Group uncertainty by task: five repeats of one task
are not five independent problem domains.

Measure the frontend, inference server and tool subprocesses separately. Include
peak RAM/VRAM, load time, prompt processing, generation throughput, browser/LSP
overhead and model-switch cost where applicable. External inference memory must
not disappear from the accounting because it is outside Alt's process tree.

Define practical acceptance targets before running each cohort. Keep a candidate
only if its measured improvement survives confirmation, produces no unacceptable
correctness/recovery regression, and has an acceptable resource tradeoff for the
intended profile. A faster invalid patch or more convincing false success is a
regression. Record mixed and negative results instead of promoting every new feature.

## Product presentation for novice users

Expose a few understandable controls: chosen model, execution access, task workflow,
speed/effort budget and optional extra tools. Show what is installed, what is active,
whether it runs locally, and the actual evidence for completion. Keep advanced
settings available without requiring novices to understand grammars or tokenizer
budgets. Preserve their draft, allow cancellation and make recovery/undo explicit.

Do not label a profile “unlimited” or “verified” because a model can emit native
tool calls. A useful label describes the workflow and its tested boundaries.
