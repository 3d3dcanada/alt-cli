# Making small local models more capable in Alt

**Research date: October 7, 2026. Status: recommendations and proposed experiments,
not implemented improvements or newly measured model results.**

The strongest opportunity is to move reliable procedural work into the harness:
locate the relevant code, present a small task, apply a precise edit, execute the
appropriate tools, and return compact, factual feedback. The model should spend
its limited capacity on the uncertain decision. It should not repeatedly reconstruct
shell syntax, copy old source perfectly, remember administrative steps, or decide
whether its own unsupported success claim is true.

Tools can expand what a small model can accomplish enormously: a compiler, browser,
database, test runner or solver supplies capabilities the weights do not contain.
They do not make an arbitrary model an unlimited reasoner. A useful goal is a
**reliable small-model system for progressively broader tasks**, with measured
success and visible remaining failures.

The paper *Small Language Models are the Future of Agentic AI* argues for small
models handling specialized, repetitive operations. It is explicitly a position
paper, not experimental proof that a tiny model can solve arbitrary tasks when
given enough tools. [Position-paper abstract][paper-small]

Keep Alt's Rust application and replaceable engine. Begin with better editing,
short executable skills, a task controller, and model-specific configuration.
Research does not justify a wholesale framework replacement, a mandatory cloud
model, or a collection of always-loaded MCP servers.

## What was examined

Discovery covered small-model harnesses, coding interfaces, structured generation,
skills, retrieval, code execution, verification, prompt optimization and training.
The acquisition record contains **38 GitHub repository snapshots, four Hugging
Face model-card snapshots, 103 repository/model files, and 11 paper abstracts**.
Inspection depth varies: selected implementations and specifications were read
closely; several peripheral candidates received documentation/license screening.
Acquisition alone is not a code audit or a successful runtime test.

[The source ledger](2026-10-07-small-model-sources.json) records repository commits,
model revisions, inspected file paths, retrieval timestamps and content hashes.
GitHub search was discovery, not evidence of quality. Stars were not a ranking
criterion. Original arXiv and several vendor/blog endpoints returned HTTP 403;
paper titles and abstracts were retrieved through Hugging Face's paper API, and
available primary documentation was inspected directly in its source repository.
Full paper methods were not independently reviewed where unavailable. Numerical
paper results below are explicitly **author-reported**, not Alt measurements.

No new weights were downloaded, no inference was run, no external skills were
installed, and no proposed capability was added to production in this research
pass. Any future live student, judge, optimizer or teacher run must honor the
owner's explicit uncensored/abliterated-model requirement. Reading a base-model
card is not permission to substitute its weights in a test.

## 1. Start with Alt's actual bottlenecks

The complete 0.6 campaigns provide a more useful starting point than a generic
agent leaderboard. See [the implementation report](../IMPLEMENTATION_0.6.md),
[Spark analysis](../evidence/v6/spark-analysis.json),
[Qwen analysis](../evidence/v6/qwen-analysis.json), and the additional
[exact-error counts](2026-10-07-alt-failure-diagnostics.json).

| Observation | Spark 1.7B abliterated | Qwen 4B Heretic | Implication to test |
|---|---:|---:|---|
| Frozen behavioral checks passed | 7/120 | 16/120 | Large reliability gap remains |
| Attempts leaving source unchanged | 36/120 | 60/120 | Getting a repair applied is itself a problem |
| Exact-text edit errors | 78 events in 29 attempts | 61 events in 27 attempts | Try a simpler edit interface |
| Missing/stale read errors | 34 events in 29 attempts | 9 events in 9 attempts | Keep freshness checks but make recovery cheaper |
| Required-plan errors | 52 events in 50 attempts | 0 | Test whether administrative work belongs in the controller |
| Engine action-budget notices | 106/120 | 1/120 | Distinguish stopping from completing |
| Deadline cancellations | 2/120 at about 240 s | 86/120 at about 360 s | More narration/steps are expensive on CPU |
| False concluding completion claims in manual review | 2 | 5 | Completion must come from evidence |

Categories overlap. These are observations, **not estimates of how many tasks a
particular change will fix**. Spark and Qwen used different checkpoints, deadlines
and reasoning settings; their aggregate scores are not a controlled comparison.
Manual claim classifications concern concluding status and selected contradictions,
not exhaustive accuracy of every sentence.

There are also real reasoning failures after successful edits. Six passing
attempts failed [additional edge cases](../evidence/v6/supplemental-counterexamples.json).
For example, a deduplication repair confused a key's hash with the key itself,
and another dropped valid `None` keys. Fixing editing friction will not by itself
repair those conceptual errors.

Three source-level details matter:

- The measured Coding focus already exposes only six Alt tools. A giant tool
  catalog did not cause these particular results. Better tool semantics and
  workflow matter more than indiscriminately reducing the count.
- [`src/engine.rs`](../../src/engine.rs) sets each reply budget to
  `min(context_tokens / 4, 2048)`. At the campaign's 8K context that is 2,048 tokens.
- [`scripts/live_acceptance.py`](../../scripts/live_acceptance.py) sets 12 engine
  turns. The prompt asks for a saved plan, current read, edit and named check.
  That makes redundant explanation, malformed edits and housekeeping costly.

The new diagnostic file was derived from retained streams using exact first-line
matches. It does not modify any original report, score, oracle or model identity.

## 2. Highest-priority change: make code editing easier

SWE-agent's central finding is that the agent's interface to a computer affects
its performance. Its paper studies an agent-computer interface for navigation,
editing and execution. That supports investigating interface design; its benchmark
scores do not establish performance for our quantized 1.7B model. [SWE-agent][swe]
[paper abstract][paper-swe]

### Concrete open-source options

**Tree-sitter and ast-grep** provide syntax trees and structural search/rewrite.
ast-grep is written in Rust and uses Tree-sitter. This fits Alt particularly well:
we can locate a function deterministically and replace its body without asking the
model to reproduce the entire old text. Syntax trees do not provide full type or
reference resolution; use a language server where those semantics are required.
[Tree-sitter][treesitter] [ast-grep][astgrep]

**Serena** supplies symbol lookup, references, symbol-body editing and language
server integration. Its inspected source exposes bounded symbol results and an
option to omit bodies, useful for compact context. Treat it as a functionality
reference or an explicitly installed optional server. **The current Serena
application is GPL-3.0-or-later; SolidLSP is separately MIT.** The upstream license
identifies a historical MIT cutoff, but a decision to reuse an old version still
requires inspecting that exact version. Do not copy current application code into
Alt under an assumed MIT license. [Serena][serena] [license overview][serena-license]

**Oh My Pi's `pi-edit`** is a real Rust implementation of snapshot/line-anchored
editing with stale-state recovery. It is MIT-licensed at the inspected snapshot.
This is more concrete than a viral editing-tool claim. Its current language also
contains block operations, registers and cross-file moves, and the crate depends
on other workspace crates. It is **not a trivial drop-in dependency or automatically
a simpler interface for Spark**. Borrow a small, tested design or evaluate the
component deliberately. Its advertised gains on other models are not an Alt result.
[pi-edit source][pi-edit] [edit instructions][hashline]

### Proposed Alt interface

Let `inspect_symbol` or a bounded read return a short handle bound to the path,
source revision and exact range. The model supplies the handle and replacement
body. Alt verifies freshness, scope and ambiguity, constructs the edit, shows the
diff, retains undo, and runs parser feedback. A stale handle yields the fresh
relevant excerpt and an actionable error.

Possible shape, **illustrative and not a current Alt API**:

```json
{"tool":"replace_symbol","arguments":{"handle":"symbol-17","new_body":"..."}}
```

Use full-file replacement as a separately tested option for genuinely small,
fully read files. Preserve encoding, final newlines, decorators, comments and
user changes. Do not use fuzzy matching that silently edits the wrong occurrence.
Hash/handle checks prevent stale writes; they do not prove that the selected
function or replacement is correct. Retain ordinary text editing for unsupported
languages and intentionally unusual edits.

**First experiment:** existing exact-text edits versus revision-bound range or
symbol edits, same model and budget. Count source correctness as well as edit
success; an easier-to-apply wrong patch is not a gain.

## 3. Skills should contain useful procedures and executable work

The Agent Skills specification packages instructions with optional scripts,
references and assets. It uses progressive disclosure: metadata first, the
selected skill body next, supporting files only when needed. It recommends fewer
than 5,000 tokens for a body; that is a broad ceiling, **far too generous as an
automatic budget for every skill in an 8K small-model session**. [Specification][skills]

For Alt, begin with short, curated skills that each do a real job:

| Proposed skill | Reliable work supplied by the harness | Model's remaining decision |
|---|---|---|
| Python test repair | Detect environment; run an existing test; normalize failure locations; preserve original checks | Diagnose and propose the smallest repair |
| Rust compiler repair | Collect `cargo check/test` diagnostics and relevant symbol excerpts | Choose a semantically correct fix |
| JavaScript package repair | Read declared package manager/module type/scripts; run the appropriate existing command | Repair the actual dependency or module mismatch |
| Browser flow check | Reusable Playwright steps, stable snapshots and assertion results | Choose the workflow and interpret unresolved behavior |
| Dependency/security review | Run pinned scanners; retain advisory/rule IDs and actual findings | Assess relevance and propose a verified correction |
| Configuration/data repair | Parse real files; validate schemas, round trips and boundary examples | Select the intended transformation |

A skill should include prerequisites, a narrow procedure, a worked example,
machine-readable outputs, known failure recovery and a verifier. Keep the reusable
procedure in code when possible. A 2,000-word motivational prompt is not a tool.

**Evidence:** the SkillsBench abstract reports 7,308 trajectories, an average
**+16.2 percentage-point** pass-rate lift from curated skills, **+4.5 points in
software engineering**, negative effects on 16 of 84 tasks, and no average benefit
from self-generated skills. It describes 86 tasks overall; retain its reported
denominators rather than inventing an explanation for the difference. Its
“smaller models” claim does not establish a result for Spark 1.7B or Qwen Heretic.
[Paper abstract][paper-skills] [benchmark repository][skillsbench]

There is an important source-quality distinction: the current repository's
`docs/paper-figures` explicitly labels its plotting data **synthetic/placeholder**.
Those figures were excluded as empirical evidence. The historical paper abstract's
claims and today's evolving repository are not interchangeable result datasets.
[Figure provenance][skillsbench-figures]

A separate AGENTS.md study reports reduced success and over 20% more inference
cost from context files in its evaluated settings, with unnecessary requirements
making tasks harder. That does not show every instruction file is harmful. Together,
these findings support **short, task-relevant, verified procedures**, not an enormous
always-loaded instruction library. [Paper abstract][paper-agents]

**Hermes Agent** is a useful implementation reference for skill discovery,
progressive loading, persistent memory and producing reusable procedures from
experience. Its self-improvement claims are not proof that blindly saving Spark's
own explanations improves Spark. Require successful execution evidence and separate
evaluation before promoting an automatically proposed skill. [Hermes][hermes]

Keep the full catalog on disk. Search metadata in the host and load only a few
relevant entries, with user-visible selection. Even 100 tokens of metadata per
skill becomes 10,000 tokens for 100 skills. Tool/skill availability stays separate
from OS access: this does not remove Full access or arbitrary user-selected tools.

## 4. Give the harness ownership of routine workflow

A useful small-model mode has explicit stages:

```mermaid
flowchart LR
    A[Goal and selected workflow] --> B[Collect relevant code and failure evidence]
    B --> C[Model proposes one change]
    C --> D[Apply with checkpoint and diagnostics]
    D --> E[Run independent checks]
    E -->|Concrete failure| B
    E -->|Required behavior verified| F[Report evidence]
```

The controller should persist the task, identify the current stage, run deterministic
housekeeping, and construct the next small request. It can save the selected
workflow as **harness-authored state**, rather than pretending the model generated
a plan. Any changed interpretation of a required plan must be explicit in the
profile and experiment; do not quietly bypass the existing requirement.

Examples of useful compound operations are “inspect this failure and its code,”
“apply this candidate and run the selected checks,” and “fetch the next relevant
diagnostic.” Each operation retains individual actions and evidence in the audit
log. It must remain cancellable, checkpoint writes and surface failed prerequisites.
The model need not generate another paragraph just to poll a running test.

Agentless is a relevant reference: localization, repair and patch validation are
separate phases. Its original results used larger hosted models; borrow the
decomposition, not its claimed score. LangGraph is another persistence/state-machine
reference, not a reason to add a Python service to a Rust desktop tool.
[Agentless][agentless] [LangGraph][langgraph]

Also keep **mini-swe-agent** as a control experiment. Its very small loop and shell
interface test whether some of our scaffold is adding unnecessary difficulty.
Its high leaderboard scores with stronger models do not mean a bash-only interface
is ideal for 1.7B. It is a comparison candidate, not a demonstrated replacement
for Goose. [mini-swe-agent][mini]

Newer directly relevant projects include **SmallCTL**, with staged execution and
failure-specific context, and **Cortex**, with cited recall and bounded working
memory. SmallCTL's inspected root supplied no license grant, so it is an idea
reference, not approved code to copy. Cortex explicitly calls its memory thesis
experimental. Neither supplies verified evidence of gains on our exact models.
[SmallCTL][smallctl] [Cortex][cortex]

## 5. Give the model compact access to knowledge, not a huge prompt

Use three complementary stores:

1. **Current task state:** goal, required checks, current source revision, selected
   skill and latest actual failure. This should be small and deterministic.
2. **Project knowledge:** symbol outlines, references, dependency/version metadata,
   source excerpts and explicitly saved decisions. Retrieve exact details on demand.
3. **Reusable experience:** short procedures that succeeded, linked to supporting
   evidence and their applicability. Invalidate stale knowledge; distinguish user
   requirements from model guesses and observed results.

Aider's repository map is a concrete design: code symbols and dependency graph
ranking under a token budget. The inspected documentation uses a nominal 1K-token
map, with dynamic expansion. Adapt the technique to Alt's existing index rather
than installing another always-on database. [Aider repo map][repomap]

Start with filename/symbol/lexical retrieval plus dependency neighbors. Measure
retrieval misses before adding embeddings or a reranker. FastEmbed-rs offers local
Rust embedding/reranking through ONNX or supported backends, but it brings another
model, model-specific licenses and RAM/CPU usage. It is an optional measured upgrade,
not a free context expansion. Under the current testing constraint, do not silently
introduce an unapproved generative router, judge or summarizer. [FastEmbed-rs][fastembed]

For unfamiliar APIs, retrieve **version-specific** documentation. Context7 provides
a useful MCP/CLI interface, but its local MCP process is a client for a service;
that does not make the documentation corpus offline or unmetered. Prefer local
installed package docs and pinned cached references for the offline path.
[Context7][context7]

Long native context is not the same as useful recall. Lost in the Middle reports
position-dependent retrieval failures and degradation with longer inputs in its
settings. RLMs instead keep large inputs outside the prompt and inspect/decompose
them through code and recursive calls. This is relevant to large logs, document
collections and repository analysis, but it assumes the model can execute the
decomposition competently. Recursive calls still consume time and tokens; the
published results do not prove effective “infinite context” for Spark on 8 GB VRAM.
[Lost in the Middle abstract][paper-middle] [RLM source][rlm]

ACE adds another useful idea: incremental, evidence-linked playbook updates instead
of repeatedly rewriting a lossy summary. Its inspected example uses DeepSeek-V3.1
for generator/reflector/curator and an 80,000-token playbook setting elsewhere.
Those defaults are a poor fit for this PC. Borrow bounded, tested memory updates;
do not copy the whole configuration or infer that “smaller” means 1.7B. [ACE][ace]

## 6. Tools that actually add capability

High-value tools compute or observe something the model would otherwise have to
guess. They are useful regardless of whether they arrive through MCP, a CLI or an
Alt-native wrapper.

| Tool family | Concrete candidates | Proposed small-model interface |
|---|---|---|
| Source structure and edits | Tree-sitter, ast-grep, language servers | File/symbol handles, bounded references, precise replacements |
| Compiler/type/lint feedback | `cargo`, Clippy, Ruff, project TypeScript tooling | File, range, diagnostic code, actual message and proposed next inspection |
| Behavioral checks | Existing test suites, Hypothesis | Deterministic outcomes, generated edge cases and minimized counterexamples |
| Browser interaction | Playwright CLI/skills or selected Playwright MCP tools | Focused accessibility observations and test assertions; no mandatory vision model |
| Dependencies and security | OSV-Scanner, selected Semgrep rules, existing Alt packs | Advisory/rule IDs, relevant source, reproduction and actual check results |
| Exact computation | Python/SQLite and, for suitable tasks, Z3 | Typed calculations/queries/constraints with machine-computed answers |
| Documentation | Installed docs, cached versioned docs, optional Context7 | Small cited excerpts for the actual dependency version |

Alt already has checks, terminal access, tool packs and optional browser/security
workflows. The next work should improve **selection, arguments, output and recovery**,
not claim these categories are all missing. Compiler success is not behavioral
correctness; scanner output needs validation; a solver proves properties of the
constraints we supplied, not of an inaccurately modeled real system.

Playwright's own current MCP README recommends considering CLI plus skills for
coding agents to avoid large schemas and verbose page snapshots. This is an
upstream design recommendation, not a measured Spark speedup. Both options still
run a real browser with its own memory cost. [Playwright CLI][playwright-cli]
[Playwright MCP][playwright-mcp]

Hugging Face's **smolagents/CodeAct** offers programmatic tool composition. Instead
of repeated model round trips for reading data, filtering it and aggregating a
result, a short program can do the sequence. CodeAct's paper abstract reports gains
across 17 models and describes an instruction-tuning dataset. This supports testing
the action representation; it does not prove that unrestricted generated Python
is easier than typed tools for every small model. [smolagents][smolagents]
[CodeAct abstract][paper-codeact]

For Alt, first offer verified scripts/recipes. Later test a bounded code-action
mode for tasks with genuinely useful composition, recording all tool work, code
errors, time and memory. Use the existing chosen execution policy. A Python REPL,
restricted imports or a separate subprocess alone is not an OS sandbox. Full access
continues to use the user's host permissions; Guided execution must keep its
actual isolation contract.

## 7. Structured decoding fixes format, not reasoning

llama.cpp supports JSON-schema-to-grammar conversion and native/generic tool
formats. Its documentation says generic formats can consume more tokens, schemas
are not all supported, and some unsupported schema features may be skipped.
It also explicitly distinguishes a decoding constraint from instructions visible
to the model. [Function calling][llama-tools] [Grammar limitations][llama-grammar]

LLGuidance is Rust-based and has an optional llama.cpp integration. XGrammar is
another structured-generation library integrated into several serving engines.
These are serving-layer capabilities, **not MCP servers that make any arbitrary
backend obey a schema**. [LLGuidance][llguidance] [XGrammar][xgrammar]

Use capability detection and actual protocol fixtures for each runtime. Prefer
simple required fields and enums. Check the real payload transmitted through
Goose; do not assume that setting `response_format` automatically constrains native
tool-call arguments or that an unsupported flag was honored. Test quotes, newlines,
Unicode, truncation, wrong tool names and invalid semantic targets separately.

The existing failures are predominantly meaningful-but-wrong operations and edits,
not just malformed JSON. Schema-valid output can still request the wrong check,
use the wrong file or implement the wrong algorithm. Constrained generation is a
useful supporting layer, not the primary explanation for the low coding scores.

## 8. Calibrate the exact Spark and Qwen profiles

The official Spark-X2.5 card says its evaluations use thinking mode and recommends
temperature 1.0, top-p 0.95 and top-k disabled. Its large native-context claim is
accompanied by a device-memory caveat. Our existing matrix used thinking **off**,
8K context, a 240-second deadline and the harness's 2,048-token reply cap.
[Official Spark card][spark-card]

The exact abliterated derivative's card recommends f16 KV cache and bounded
reasoning; its example uses a 2,000-token reasoning budget within a 4,096-token
generation. The publisher reports that thinking can consume thousands of tokens.
These are publisher recommendations, not independent guarantees for our runtime.
Do not blindly copy its 65K-context/GPU example onto a GTX 1070.
[Pinned derivative card][spark-derivative]

An important integration trap follows: **a 2,000-token thinking budget combined
with Alt's current 2,048-token total reply allowance can leave almost no room for
the actual action.** Test bounded reasoning and total output together; record
whether the runtime counts reasoning against the output budget. Reconcile context,
prompt, reasoning and action headroom instead of merely exposing a thinking toggle.

Qwen3-4B-Instruct-2507 is explicitly a **non-thinking** model according to its
official card. Template-default in our Qwen cohort should not be described as an
enabled reasoning mode. Its suggested sampling values also differ from Spark's.
Inspect the derivative/template path instead of applying one profile to both.
[Qwen card][qwen-card]

Useful controlled experiments, in order:

- Log effective template/parser, payload, sampling defaults, context and output
  budgets. Preserve the exact selected weights and check native tool round trips.
- On Spark, compare thinking off with a **bounded** reasoning allocation while
  preserving sufficient action space and the same total compute/time constraints.
- Compare Q4 with Q6/Q8 from the same derivative when it fits. Quantization can
  change behavior; do not assume a higher bit rate always fixes a task.
- Measure 4K/8K context for usefulness and cost before increasing it. Use the
  actual GPU/driver/runtime on the PC for VRAM and throughput claims.
- Test action/turn budgets and concise operational prompts as separate factors.
  More allowed turns cannot help if wall time is spent on repetitive prose.

The reference design should load one generative model at a time, retain CPU
operation, avoid mandatory vector/browser/agent servers, and run multiple candidate
attempts sequentially on the 8 GB card. RAM and VRAM remain separate budgets.
Speculative decoding is mainly a throughput technique with extra resource and
compatibility costs; it does not inherently improve the target model's reasoning.

## 9. Learning, retries and training: valuable after the interface works

**Verifier-guided repair:** return concrete failures, affected code and the last
applied diff. Try another candidate only when there is a testable hypothesis or
useful new evidence. Preserve alternatives in disposable snapshots and select
using independent checks. A selected best-of-three result must be reported with
all three attempts and their cost, not as a first-attempt success.

**Self-review needs external evidence.** The self-correction paper distinguishes
intrinsic self-correction from correction with outside feedback, reporting that
the former can fail or degrade reasoning. A second instance of the same small
model is not an independent correctness oracle. Compilers, tests, counterexamples
and factual observations should drive repair. [Paper abstract][paper-correction]

**DSPy/GEPA** can optimize prompts or other textual parameters against an evaluator
offline. That is an attractive use of our retained failure traces. However, GEPA's
examples use a separate reflection model and many evaluations; developer-side
optimization is not free runtime intelligence. Use an explicitly selected allowed
optimizer model, development tasks and a fresh validation set, and export only a
small versioned artifact after it wins a held-out comparison. Do not tune on our
already-inspected historical held-out cases and call them unseen.
[DSPy][dspy] [GEPA][gepa] [Paper abstract][paper-gepa]

**Fine-tuning/LoRA and agent RL** may teach the exact tool language and recovery
behavior. Agent Lightning records trajectories through a gateway and trains with
real harnesses; CodeAct provides another tool-use-training reference. Unsloth is a
candidate training toolchain. This belongs after reliable fixtures and sufficient
verified trajectories exist. Our current 23 passing frozen-check attempts include
known coverage gaps: they are not a ready-made high-quality training corpus.
[Agent Lightning][lightning] [Unsloth][unsloth]

Serving a Q4 model on 8 GB VRAM does not establish that training it fits. Training
requires a separately qualified stack and resource budget; do not prescribe modern
CUDA examples to a Pascal GPU without checking compatibility. Dataset licenses,
teacher/output terms and derivative-model licenses also need review before
redistributing a trained model.

Do not begin with an always-running multi-agent council, unbounded recursive
calls, automatic self-written skills, huge pasted knowledge bases or a generic
“think harder” prompt. Each can consume the scarce resources without resolving
the observed failure. Specialized serial stages can share the same loaded model.
Optional larger or remote models must remain explicit user choices, never a hidden
dependency of a claimed local-small-model result.

## 10. Reuse shortlist and licensing

These are inspected project-level licenses at captured revisions, not blanket
clearance for every dependency, model, ruleset or task asset. Keep notices and
review the precise component before copying or shipping it.

| Candidate | Inspected license | Decision for Alt |
|---|---|---|
| ast-grep / Tree-sitter | MIT / MIT | First-choice structural-code components |
| Oh My Pi `pi-edit` | MIT; workspace dependencies | Evaluate a narrow editing component/design; do not adopt the entire app |
| Serena / SolidLSP | GPL-3.0-or-later app / MIT library | Optional server or separate MIT component; license split matters |
| Agent Skills standard | Apache-2.0 repository | Adopt compatible discovery/packaging; individual skills have separate terms |
| Hermes Agent | MIT | Study skill/memory integration; require evidence before learned-skill promotion |
| Aider | Apache-2.0 | Reuse repository-map ideas; optional architect/editor comparison |
| smolagents | Apache-2.0 | Code-action experiment/reference, not a mandatory Python runtime |
| llama.cpp / LLGuidance / XGrammar | MIT / MIT / Apache-2.0 | Runtime capability integration and qualification |
| mini-swe-agent / Agentless | MIT / MIT | Minimal-loop and staged-workflow experimental controls |
| DSPy / GEPA | MIT / MIT | Offline optimization with frozen evaluation boundaries |
| ACE | Apache-2.0 | Bounded evidence-linked playbook updates; discard unsuitable large defaults |
| RLM | MIT | Later large-input analysis experiment with accounted recursive cost |
| FastEmbed-rs | Apache-2.0 library | Optional local retrieval; inspect each embedding model separately |
| Context7 | MIT client | Optional online docs; not an offline knowledge database |
| Playwright CLI / MCP | Apache-2.0 / Apache-2.0 | Compare concise CLI skills with selected native tools |
| [Ruff][ruff] / [OSV-Scanner][osv] | MIT / Apache-2.0 | Useful deterministic diagnostics and dependency checks |
| [Hypothesis][hypothesis] | MPL-2.0, exceptions noted upstream | Generate boundary cases and minimize counterexamples |
| [Z3][z3] | MIT | Optional exact constraint solving for suitable domains |
| [Semgrep][semgrep] | LGPL-2.1 code; rules/services separately licensed | Improve existing integration; inspect selected rules and distribution terms |
| Agent Lightning / Unsloth | MIT / Apache-2.0 | Later offline training infrastructure, separately resource-qualified |
| Cortex | MIT | Experimental working-memory reference |
| SmallCTL / Kutty | No license grant established in inspected snapshot | Discovery/watchlist only; no code reuse recommendation |

## Recommendation and proposed next step

Build the next experimental Alt profile around **revision-bound edits, a short
curated skill, host-managed routine steps, compact real feedback, and a properly
measured model configuration**. Add repository maps and versioned documentation
next. Test structured generation where the provider supports it. Save expensive
search, prompt evolution and weight training for evidence that the simpler changes
have reached their limit.

The [experiment/work-order proposal](2026-10-07-small-model-experiments.md) turns
this into isolated changes, controls, resource accounting and pass/fail criteria.
It keeps current behavior as the baseline and makes no numerical improvement
promise before those experiments run.

[swe]: https://github.com/SWE-agent/SWE-agent/tree/3ea751c087f32b16e039a2233dd6eefecef325d5
[paper-swe]: https://huggingface.co/papers/2405.15793
[treesitter]: https://github.com/tree-sitter/tree-sitter/tree/752c612a1359f00e4c113593837a0c1e880214e3
[astgrep]: https://github.com/ast-grep/ast-grep/tree/029430eac79159575b36d35d75886e9c8ef81f00
[serena]: https://github.com/oraios/serena/tree/3b99f8b024dafd58c962ea6e74f37c8a730ef532
[serena-license]: https://github.com/oraios/serena/blob/3b99f8b024dafd58c962ea6e74f37c8a730ef532/LICENSE
[pi-edit]: https://github.com/can1357/oh-my-pi/tree/04c267c37a940b391e8c73c528d9fd47bcb444e6/crates/pi-edit
[hashline]: https://github.com/can1357/oh-my-pi/blob/04c267c37a940b391e8c73c528d9fd47bcb444e6/crates/pi-edit/prompts/hashline.md
[skills]: https://github.com/agentskills/agentskills/blob/69ef37e9424c0a7ea9dd2293b559e43ec8176379/docs/specification.mdx
[paper-skills]: https://huggingface.co/papers/2602.12670
[skillsbench]: https://github.com/benchflow-ai/skillsbench/tree/9a1f4dd5f7659f75707435da3ce854b6e48321d1
[skillsbench-figures]: https://github.com/benchflow-ai/skillsbench/blob/9a1f4dd5f7659f75707435da3ce854b6e48321d1/docs/paper-figures/README.md
[paper-agents]: https://huggingface.co/papers/2602.11988
[hermes]: https://github.com/NousResearch/hermes-agent/tree/0e37a439bda15ef3c28a4d20593964d7c6a527a6
[agentless]: https://github.com/OpenAutoCoder/Agentless/tree/5ce5888b9f149beaace393957a55ea8ee46c9f71
[langgraph]: https://github.com/langchain-ai/langgraph/tree/39c523eb0af1d192f739fd2b7ddfea38f3002a2a
[mini]: https://github.com/SWE-agent/mini-swe-agent/tree/04d809ceab9df28f9adaed044884180159172930
[smallctl]: https://github.com/lowspeclabs/SmallCTL/tree/1ce1cb556e9c0a2fe95a5bb5c6bb0287823f33b6
[cortex]: https://github.com/dereksantos/cortex/tree/d7ab1a4488c38ca0144e16f824918b6a5cfeab1f
[repomap]: https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/website/docs/repomap.md
[fastembed]: https://github.com/anush008/fastembed-rs/tree/29059745b8c5df6a1adc263b6903609e164ad3c5
[context7]: https://github.com/upstash/context7/tree/72181dd4ac5ba5761ebcb3e4d04566e819293db6
[paper-middle]: https://huggingface.co/papers/2307.03172
[rlm]: https://github.com/alexzhang13/rlm/tree/d04208afbad29ca675ab13478c40ee8bebc84bfe
[ace]: https://github.com/ace-agent/ace/tree/82709de050e1db6e6ef2f07bcb0393560b94992a
[playwright-cli]: https://github.com/microsoft/playwright-cli/tree/b85c7a736bb473bf55b584e54a09ffa698d6d871
[playwright-mcp]: https://github.com/microsoft/playwright-mcp/tree/f183dad4a52965583e3cc1d59b88cdc279e2e57d
[smolagents]: https://github.com/huggingface/smolagents/tree/96f33faaf028479119ec8d34507b47694cf14e34
[paper-codeact]: https://huggingface.co/papers/2402.01030
[llama-tools]: https://github.com/ggml-org/llama.cpp/blob/c479922ac520a08969b4c1dc154d7bbb3c386d85/docs/function-calling.md
[llama-grammar]: https://github.com/ggml-org/llama.cpp/blob/c479922ac520a08969b4c1dc154d7bbb3c386d85/grammars/README.md
[llguidance]: https://github.com/guidance-ai/llguidance/tree/e7c69004694064a01db30ddc7a29a103a5d2f3c7
[xgrammar]: https://github.com/mlc-ai/xgrammar/tree/ed1bc8312ba37947050938bf8bdd88b923a1057b
[spark-card]: https://huggingface.co/XHToken/Spark-X2.5-1.7B/blob/14d6e83c13c7add2b62a7c39b2131f4ed1cddcf8/README.md
[spark-derivative]: https://huggingface.co/darioooooo0o/Spark-X2.5-1.7B-Abliterated-GGUF/blob/0193e8f9b55c7f8e9c58799694696a17d8f6c6c5/README.md
[qwen-card]: https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507/blob/cdbee75f17c01a7cc42f958dc650907174af0554/README.md
[paper-correction]: https://huggingface.co/papers/2310.01798
[dspy]: https://github.com/stanfordnlp/dspy/tree/bc8af7ed3d5ee0b892211e5c03274396442a413c
[gepa]: https://github.com/gepa-ai/gepa/tree/fb1ed589fd83372caef499cffc2c73173d3b096b
[paper-gepa]: https://huggingface.co/papers/2507.19457
[lightning]: https://github.com/microsoft/agent-lightning/tree/d381995396274039f2bb1cbe5ff42ac8067f4e47
[unsloth]: https://github.com/unslothai/unsloth/tree/0587140f4147855939d048075ce9f662a141d7ad
[paper-small]: https://huggingface.co/papers/2506.02153
[ruff]: https://github.com/astral-sh/ruff/tree/c33cdaad46019d31cbbd00168b0c76fdbcb6cad9
[osv]: https://github.com/google/osv-scanner/tree/1b86129be1f0285c403616cb362ac188de676c6e
[hypothesis]: https://github.com/HypothesisWorks/hypothesis/tree/ca2a4c6e3c6b979d9fbb5ef9db614885a736d2aa
[z3]: https://github.com/Z3Prover/z3/tree/8f621dca182c6d3c04d0eb52b123cb3eb138309a
[semgrep]: https://github.com/semgrep/semgrep/tree/c9ebd1a0ead40f81302c72d60cc73c444a4afe87
