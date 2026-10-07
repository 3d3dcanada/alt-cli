# Strengthening reasoning in small local models

**October 7, 2026. Research and proposed experiments. No new inference,
training, model-weight downloads, or production changes were performed.**

There are credible ways to improve the reasoning of a small-model system, and
credible ways to improve the model's weights. The practical combination for Alt
is **bounded reasoning, decomposition, executable observations, verified search,
and eventually distillation into the exact tool interface**. A stronger trained
small model can remain local after the training work is finished elsewhere.

This follows [the harness research](2026-10-07-small-model-harness.md). That report
addresses editing, skills, context and tools. This review asks a narrower question:
how to improve the quality of difficult decisions, rather than merely make the
same decision easier to execute.

## Research scope and evidence

Live discovery used GitHub repository search and Hugging Face paper/model search.
This acquisition set contains **26 repository snapshots, 19 model-card snapshots,
two dataset cards, 100 repository/model/dataset files, and 33 paper abstracts**.
The 100 files include **three full-text upstream PDFs**: DeepSeek-R1,
VibeThinker-1.5B, and VibeThinker-3B. Selected method, evaluation and limitation
sections of those reports were reviewed, alongside selected source implementations.
Other papers were reviewed through their author abstracts, not full methods.

Google, Bing, DuckDuckGo, arXiv HTML and the attempted OpenReview endpoint returned
HTTP 403 in this environment. Hugging Face's search/API, two relevant blog pages,
GitHub APIs and upstream GitHub-hosted PDFs were accessible. This is substantive
public-web research, but not an unrestricted search-engine crawl or an exhaustive
literature review. [The source ledger](2026-10-07-reasoning-sources.json) records
queries, failures, revisions, paths and SHA-256 hashes. Acquisition depth varies;
none of these projects has been completely audited or independently reproduced.

All numerical results attributed to outside work below are **author-reported**.
They are not measurements of Alt, Spark's abliterated checkpoint, its GGUF
quantization, or the GTX 1070. The retained Alt campaigns remain our only cited
Alt measurements. Every future live student, teacher, critic or router evaluation
must use an explicitly selected uncensored/abliterated checkpoint, consistent with
the owner's instructions. Model-card research does not substitute base weights in
a live run.

## 1. Evidence that small models can become better reasoners

| Primary source | What was reported | What it supports for Alt | Important boundary |
|---|---|---|---|
| DeepSeek-R1, section 2.4 | SFT on 800K curated samples distilled reasoning into models including 1.5B; the released distills did not include their own RL stage | Reasoning behavior can be transferred into small weights | The 1.5B model's Table 5 LiveCodeBench result is 16.9%; strong math does not imply a reliable coding agent |
| DeepScaleR-1.5B card | AIME24 rose from 28.8% to 43.1% after RL; 40K problem/answer pairs | Further training can improve an already distilled 1.5B model | Math-specific; training used 8 then 32 A100-80GB GPUs and progressively longer contexts |
| Open-RS paper/card | A 1.5B model trained with 7K samples reached 46.7% on AIME24 and 80% on AMC23 | Smaller curated training projects can produce gains | Four A40-48GB GPUs over 24 hours; published evaluation permits 32K output tokens; quoted rental cost is not our budget |
| VibeThinker-1.5B technical report | Diversity-oriented SFT followed by verified-reward RL; 51.1 on its LiveCodeBench-v6 subset | Coding reasoning is also trainable at 1.5B | Up to 40K output tokens, 8 repeated coding samples, comparison scores from other reports; v6 subsets differ |
| SOD repository/paper | For Qwen3-1.7B, LiveCodeBench average@32 rose from 22.73 for vanilla to 40.63 for SOD; the 0.6B row rose from 14.89 to 27.72 | Directly relevant small-model tool-integrated training exists | Author results use different training/evaluation conditions from Alt; training recipe uses eight H20-96GB GPUs and 20,480 response tokens |
| rStar-Math abstract | Search plus a learned process verifier and self-evolved training improved Phi3-mini-3.8B on MATH from 41.4% to 86.4% | Model/search/verifier combinations can outperform the starting model substantially | Math, multiple models, extensive search and millions of synthesized solutions; not an 8GB turnkey harness |

[DeepSeek full report][r1-pdf], [DeepScaleR card][deepscaler-card],
[Open-RS abstract][paper-openrs] and [recipe][openrs-config],
[VibeThinker-1.5B full report][vibe15-pdf], [SOD results][sod],
[rStar-Math abstract][paper-rstar].

The SOD repository says **average@32**: an average of repeated generations.
Do not present that as an oracle selecting the best of 32, a 32-attempt success
probability, or a guaranteed first-run outcome. VibeThinker's ordinary repeated
pass-rate estimates and its separate candidate-selection enhancement also need
different labels.

These results make training a real development path. They do not establish that
we can install a reasoning plugin and obtain the same gains from arbitrary weights.
Spark itself already underwent reasoning and agentic post-training according to
its official card. The immediate task is to expose that capability properly and
measure where additional training is still needed. [Spark card][spark-card]

## 2. Use additional inference compute where it can help

### Bounded native thinking

First validate the model's native reasoning template, parser, supported budget
controls and actual transmitted request. Spark's official examples use a Spark
tool parser and Qwen3 reasoning parser; its published evaluations use thinking.
That does not prove every GGUF runtime exposes equivalent controls.

Alt's current [`src/engine.rs`](../../src/engine.rs) sets
`GOOSE_MAX_TOKENS = min(context_tokens / 4, 2048)`. At 8K context, each reply is
capped at 2,048 tokens. A reasoning allocation of 2,000 inside that shared cap can
leave almost no space for the actual tool call or answer. The existing Spark
campaign disabled thinking. Both facts require a controlled profile experiment.
They do not explain all its failures, or prove enabling thinking will improve it.

Start by measuring bounded allocations such as 0, 256, 512 and 1,024 thinking
tokens, subject to real runtime support and sufficient action/prompt headroom.
These are **candidate budgets**, not optimal settings. If a provider cannot enforce
or account for thinking separately, report that and compare supported total-output
limits. A prompt saying “use 512 tokens” is not a serving-layer guarantee.

The relevant unit is a completed, checked operation within a wall-time budget.
An excellent unfinished explanation or a truncated tool call is a failure.
Qwen3-4B-Instruct-2507 is non-thinking; a different thinking checkpoint is a
model change, not an on/off ablation of that same checkpoint. See the prior
[profile analysis](2026-10-07-small-model-harness.md#8-calibrate-the-exact-spark-and-qwen-profiles).

### Adaptive effort

The test-time-compute study finds that the best strategy depends on problem
difficulty and that gains depend on the smaller model already having non-trivial
success. This supports spending more effort on a promising unresolved task, rather
than blindly extending every response. [Author abstract][paper-ttc]

**DART** is a relevant recent MIT reference. It samples cheap non-thinking drafts,
uses agreement to decide whether to invoke thinking, and probes reasoning budgets.
Its abstract describes a routing signal extending to 0.6B; the inspected README's
main numerical table concerns 8B/14B/32B and hosted models. Neither supplies a
measured Spark result. [DART][dart] [abstract][paper-dart]

Its current reference implementation also needs qualification:

- The supplied model client is abstract plus a mock. Real runtime integration is
  the adopter's work; importing the router does not implement reasoning budgets.
- The budget router's `_all_equivalent` skips empty subsequent drafts. For a
  non-empty first answer and empty second answer, that predicate can accept
  agreement. Treat missing/truncated/invalid candidates as distinct outcomes.
- A branch named `VERIFIED_THINK` checks agreement with another model answer.
  Agreement is a confidence signal, not external correctness evidence.
- The supplied code-answer equivalence compares normalized text; it does not
  execute candidate programs. The abstract's execution-based-equivalence gains
  require an additional evaluation implementation to reproduce.
- Count draft and retry tokens as well as thinking tokens. Reduced thinking-token
  usage alone does not establish lower total latency or GPU work.

These are observations about the inspected source, not an audit of the paper's
experiments. [Client][dart-client] [budget router][dart-budget]
[answer equivalence][dart-equivalence]

For Alt, an initial effort policy should use observable signals: an actual failed
check, an unresolved ambiguity, missing documentation, a truncated action, or lack
of progress. Compare it with fixed effort before promoting it. Self-reported
confidence and draft agreement should never determine verified completion.

## 3. Search over executable candidates

### The useful part of S*

S* combines candidate generation, sequential repair and selection through
distinguishing inputs. Its abstract reports gains across 12 models, including
a 3B model. This is more relevant to Alt's coding work than a generic “reflect
harder” instruction. [S* abstract][paper-sstar] [implementation][sstar]

However, the inspected 3B experiment script generates 16 candidates across up to
three rounds and uses **GPT-4o-mini as the test generator**. The driver also creates
judge models and offers an oracle-selection condition. Its scores should not be
presented as results from a single offline 3B agent. Oracle selection measures
available coverage using privileged evaluation information, not a deployable
selection method. [3B script][sstar-script] [driver][sstar-driver]

The practical Alt adaptation is much smaller:

1. Generate one repair in a disposable snapshot and run the declared checks.
2. If concrete evidence justifies it, produce a second or third distinct candidate,
   sequentially, within the **same total** effort budget.
3. Compare candidates using trusted contracts, existing independent checks and
   additional permitted counterexamples. Retain every failure and its cost.
4. Return the selected patch and what was checked. If checks cannot distinguish
   the candidates, show that uncertainty rather than manufacturing a winner.

For example, two deduplication implementations can both pass normal integer
examples while differing on colliding hashes or valid `None` keys. Host-supplied
properties and checked boundary cases can separate them. Those examples are now
known development cases from Alt's existing evidence, not fresh held-out tests.

A generated input that makes candidates disagree is useful evidence of a
difference. It is **not automatically evidence of which answer is correct**.
The expected output needs a trusted contract, reference calculation, reviewed
property or other independent source. A weak model writing both repair and oracle
can make the same mistake twice.

Sequential sampling can use one loaded model. It still costs additional time.
Candidate errors can be highly correlated. The idealized probability
`1 - (1 - p)^k` assumes independent attempts and perfect selection; it is not a
prediction to derive from Spark's historical aggregate score.

### Tree search and proxy frameworks

Tree of Thoughts, rStar and related search approaches explore branches and rank
intermediate states. The general technique is useful when branches represent
meaningful alternatives and ranking signals are reliable. The inspected original
Tree-of-Thoughts loop uses model-based values/votes; it does not turn those values
into objective checks. [Tree-of-Thoughts code][tot-code]

**OptiLLM** and **ThinkBooster** provide open-source experimentation infrastructure
and OpenAI-compatible interfaces. They are worth comparing deliberately, not
inserting as an automatic intelligence upgrade. Native tool calls, streaming,
cancellation, provider controls and all secondary inference calls still need
qualification. [OptiLLM][optillm] [ThinkBooster][thinkbooster]

One concrete source distinction: OptiLLM's inspected `mcts.py` generates possible
assistant replies, asks the model what the user might say next, and asks the same
model to score conversation quality. That is a search over simulated dialogue,
not a search over repository revisions checked by a compiler/test runner.
Its existence does not prove better repairs. [Inspected MCTS][optillm-mcts]

Implement Alt's initial search over **real task states**: source hashes, patches,
actual check outcomes and observed facts. Keep branching shallow. Where a task
has no reliable automatic verifier, provide evidence-backed alternatives and
explicit user judgment rather than self-awarded correctness.

## 4. Make each reasoning problem smaller and better grounded

Least-to-most prompting decomposes a problem into easier subproblems and carries
their answers forward. Its prominent result used GPT-3 on SCAN, not a local tiny
coding model. ReAct interleaves reasoning and observations. These support testing
decomposition and grounding; neither establishes an optimal universal prompt.
[Least-to-most abstract][paper-ltm] [ReAct abstract][paper-react]

For Alt, use a short decision packet:

- The immediate question and relevant requirement.
- A bounded source excerpt or version-specific documentation.
- What was observed, with an evidence reference and source revision.
- A candidate hypothesis and the next operation that could test it.
- The remaining work and effort budget.

The controller maintains those facts across steps. The model proposes the
uncertain decision. Compilers, interpreters, browser assertions and numerical or
symbolic tools perform the parts that can be computed exactly. This reduces both
reasoning burden and the propagation of invented intermediate facts.

Examples include computing a boundary result in Python, querying data in SQLite,
checking a dependency version against installed documentation, or testing a
configuration constraint with Z3. The model still has to choose the right
calculation or formalization. Incorrect constraints do not become true because a
solver accepts them.

The paper *Replacing thinking with tool usage enables reasoning in small language
models* studies **training** models up to 3B on multi-turn interactions with a
stateful tool/DSL for Python repair. This is directly relevant to our hypothesis.
It does not show that installing an arbitrary tool reproduces the effect without
training on its interface. [Author abstract][paper-tool-reasoning]

**Plan-and-Budget** supplies a decomposition/budgeting implementation reference.
Its abstract's “smaller” example is 32B versus 70B. The inspected local-allocation
code includes word-budget instructions; those are not hard token constraints.
Use its design as a hypothesis for Spark, rather than adopting its large-model
results as small-PC evidence. [Project][planbudget] [abstract][paper-planbudget]

Knowledge gaps need retrieval. More thinking cannot recover a library change
that the model never learned. A recent FlyBy paper distinguishes execution from
knowledge bottlenecks, but its proposed solution queries stronger models.
It therefore introduces a separate inference dependency. For the default local
path, try installed/versioned docs and executable observation first; an external
model remains an explicitly selected option. [FlyBy abstract][paper-flyby]

## 5. Train reasoning that fits the student's capacity

### Distillation is the first serious weights experiment

Distillation trains the student on examples from a stronger or better-performing
teacher. It can transfer procedures and knowledge that repeated self-questioning
does not supply. The teacher can run during development; it need not become a
dependency of the resulting local application.

For Alt, curate examples of the **actual tool language and workflow**: inspect a
failure, identify the relevant symbol, propose a precise change, execute it, read
the real check result and recover from a failed hypothesis. Include examples where
the right action is to retrieve missing facts or stop an unproductive repetition.
Keep procedures short enough to complete under the intended deployment budget.

Use successful, independently checked actions; retain failed trajectories as
failure/recovery examples with accurate labels. Do not relabel a plausible
explanation or a passing incomplete oracle as proof. Where appropriate, mask user
and tool-result tokens in supervised training and train assistant decisions and
tool calls, following the design described in the SmolLM3 recipe. Tool observations
must come from execution, not teacher-authored fictional results. [SmolLM3 recipe][smollm-blog]

### Long traces can make a small model worse

*Small Models Struggle to Learn from Strong Reasoners* reports that models at or
below 3B can benefit more from shorter/simpler reasoning or mixed-length
distillation. *Through the Valley* reports substantial degradation from limited
long-CoT training in its settings, including some very small models that do not
recover after much more data. *The Valley of Code Reasoning* describes non-monotonic
training behavior and benefits from easier problems in low-data regimes.
[Learnability-gap abstract][paper-gap], [long-CoT abstract][paper-valley],
[code-distillation abstract][paper-code-valley].

This does not establish that shorter is always better. VibeThinker-3B deliberately
uses long high-quality traces in later training stages. The takeaway is to fit
data difficulty, representation and length to the student's learning/deployment
capacity, then measure. Copying a 40K-token teacher transcript into a 1.7B training
set is not a guaranteed upgrade.

### SOD is an unusually relevant follow-on

SOD supplies teacher feedback on student-generated tool trajectories, adjusting
its strength at each step when the student and teacher diverge. This addresses a
specific problem: a bad tool call creates a bad observation, and subsequent teacher
supervision becomes less useful. The inspected code provides training scripts,
not just a prompt library, with 0.6B and 1.7B result rows. [SOD][sod]

Use it as a later experiment after straightforward supervised tool/reasoning
training works. It requires student and teacher inference, log probabilities,
training infrastructure, executable environments and substantial resources.
The released datasets are useful research references, but the inspected 3K/30K
cards do not establish a dataset license grant; their mixtures include separately
sourced data. Code's Apache license is not automatic clearance for those datasets.
[3K dataset card][data3k] [30K dataset card][data30k]

### Verified-reward RL can improve the learned policy

After SFT, reward actual verified outcomes and useful recovery. Frameworks include
Open-R1, veRL and rLLM. A correctness-first reward can favor efficient successful
solutions while preventing a short invalid answer from winning on length alone.
Keep format compliance separate from behavioral correctness. [Open-R1][openr1]
[veRL][verl] [rLLM][rllm]

Do not copy reference rewards without reviewing them. In Open-RS's inspected
`rewards.py`, unparseable gold answers yield reward `1.0` rather than an explicit
excluded observation, and the cosine length reward uses Python string length.
That is not a ready-made token-accounted Alt verifier. Validate training examples,
measure tokens with the actual tokenizer, and test reward behavior on wrong,
missing, malformed and incomplete outcomes. This does not independently invalidate
the paper's reported results. [Reward implementation][openrs-rewards]

The literature is also divided on whether a particular RL setup expands reasoning
coverage or mainly makes already-possible correct outputs more likely. One study
finds improved low-k performance but narrower high-k coverage; ProRL reports
expanded coverage with prolonged training and other controls. Both favor reporting
first-attempt success, available coverage, actual selection and total training cost,
rather than declaring universal self-improvement. [RLVR study][paper-rl-limit]
[ProRL abstract][paper-prorl]

## 6. Reasoning-model candidates, including abliterated variants

The table identifies **research/qualification candidates**, not tested additions
to Alt's catalog. A derivative's label does not establish its retained reasoning,
tool competence or performance. Exact revisions are in the ledger; all future
live evaluation must preserve the selected checkpoint and artifact hashes.

| Family | Why it is relevant | Qualification issue | Inspected abliterated/Heretic candidate |
|---|---|---|---|
| Spark-X2.5-1.7B | Existing selected model; reasoning and agentic post-training | Native thinking/output headroom and runtime/parser behavior | Existing Spark derivative from prior study |
| SmolLM3-3B | Dual reasoning modes and documented XML/Python tool formats | Verify derivative template and actual GGUF engine's native tool round trips | [Huihui-SmolLM3 GGUF][smollm-derivative] |
| DeepScaleR-1.5B | Small weights with demonstrated published post-training math gains | Math specialization and long-output evaluation do not imply tool competence | [DeepScaleR abliterated GGUF][deepscaler-derivative] |
| VibeThinker-1.5B | Reasoning-specific math/code post-training; quantized derivative exists | 40K-token benchmark budget is a poor initial PC setting; GGUF card omits a license field | [VibeThinker Heretic GGUF][vibe15-derivative] |
| VibeThinker-3B | Strong verifiable-reasoning training reference based on a code model | Official card explicitly says it was **not trained for tool calling/agent coding**; derivative fine-tune needs evidence | [Heretic function-calling fine-tune][vibe3-fc] |
| LFM2.5-1.2B-Thinking | Small reasoning model with documented tool syntax and llama.cpp path | Different architecture/parser and custom LFM Open License | [Huihui LFM thinking GGUF][lfm-derivative] |
| Nanbeige4.1-3B | General reasoning and reported agentic post-training | Its published deep-search setup uses Qwen3-14B-thinking summaries and online tooling | [Nanbeige Heretic][nanbeige-derivative] |
| Phi-4-mini-reasoning, 3.8B | Explicit small math-reasoning checkpoint | Official card says designed/tested for math only; not an automatic general-agent replacement | Abliterated listings found; no derivative card qualified in this pass |
| Qwen3-0.6B/1.7B and 4B-Thinking-2507 | Native reasoning families; useful training/reference comparisons | Distinguish Thinking from the existing non-thinking Instruct-2507 family | Listings found; exact current selected weights remain the baseline |
| MiMo-7B-RL | Relevant to the owner's MiMo interest; math and code post-training reference | Larger memory/latency footprint; 7B weights and KV still need qualification on 8GB | Base card only in this pass; no live test or replacement |

[SmolLM3 card][smollm-card], [VibeThinker-3B card][vibe3-card],
[LFM card][lfm-card], [Nanbeige card][nanbeige-card],
[Phi card][phi-card], [MiMo card][mimo-card].

The VibeThinker-3B Heretic `-fc` card specifically claims fine-tuning on
`NousResearch/hermes-function-calling-v1`. Its documentation is sparse and contains
no retained-reasoning/tool evaluation. That is a useful candidate for a protocol
and behavioral qualification, not proof that the upstream model's reasoning
scores carry over. The official upstream caveat must remain visible.

The VibeThinker-3B full report's additional **CLR** result generates **32 candidate
trajectories**, extracts five claims per trajectory and uses the same model as a
self-verifier. That enhancement is a separate selection procedure, not one cheap
3B generation. Its standard benchmarks also allow long outputs; the card suggests
60K–100K output tokens for a particularly difficult math set. Do not use either as
a GTX 1070 responsiveness claim. [Full report, evaluation section][vibe3-pdf]

Nanbeige's 14B summarizer is an especially important accounting detail: a 3B
headline does not mean the published deep-search system uses only that 3B model.
A local adaptation must separately qualify any summaries/retrieval changes.

## 7. What fits the reference PC

Retain one loaded generative model, CPU operation, and deterministic CPU tools.
Start with useful 4K/8K contexts and sequential candidates. Larger context raises
KV and prompt-processing cost; reasoning and actions also consume output/context
headroom. Weight download size is not peak VRAM, and GPU memory is not system RAM.

The inspected quantizer cards list approximately 1.1GB for VibeThinker-1.5B Q4_K_M,
1.2GB for DeepScaleR Q4_K_M and 2.0GB for SmolLM3 Q4_K_M. Those are publisher-listed
artifact sizes, not our downloads, verified quality rankings or memory measurements.
They make these candidates worth qualifying; they do not establish generation
speed or successful GPU loading on Pascal. Q6/Q8 may be feasible for some candidates,
but compare their behavior and cost rather than assuming bit rate proves quality.

Train elsewhere first if a weights experiment proceeds. “Small model” describes
parameter count, not necessarily training memory: Open-RS uses 192GB aggregate
GPU VRAM; SOD's documented setup uses 768GB aggregate GPU VRAM. Even TinyZero's
single-GPU label does not identify an 8GB GTX 1070 configuration. Its README also
says its 0.5B base experiment failed to learn reasoning and meaningful 3B experiments
used two GPUs. The repository is now deprecated in favor of veRL. [TinyZero][tinyzero]

A future lightweight SFT/LoRA recipe needs its own qualified hardware/software
budget. Q4 inference fit does not establish training fit; modern bfloat16,
FlashAttention/vLLM and CUDA examples cannot be assumed compatible with the 1070.
Exporting a trained model or merged adapter for local inference is a separate,
testable step.

## 8. Promising research that is not a runtime plugin

- **s1 budget forcing:** extends or stops a specially trained model's thinking.
  Its main experiment fine-tunes Qwen2.5-32B on 1K examples. Appending “Wait” to
  arbitrary small models is not the demonstrated intervention. [Abstract][paper-s1]
- **Learned process verifiers:** can improve search, but are another model and
  another possible source of correlated error. Start with executable checks;
  use PRM800K as a math process-supervision reference, not a general software
  correctness database. [PRM800K][prm800k] [paper abstract][paper-prm]
- **Coconut:** feeds continuous hidden states back as input embeddings. It requires
  training and inference changes, not an ordinary OpenAI/MCP wrapper. Its reported
  improvements concern particular reasoning tasks. [Source][coconut]
  [abstract][paper-coconut]
- **HRM/TRM:** small recurrent networks for structured puzzle problems, including
  Sudoku, mazes and ARC. TRM's 7M size is evidence of task-specific efficiency,
  not a general text assistant that can be attached to Spark. The inspected TRM
  README's smallest stated training example uses an L40S-48GB for roughly 18 hours,
  and the repository is archived. [TRM][trm] [abstract][paper-trm]
- **Efficient Reasoning on the Edge:** relevant adapter-switching/KV-sharing ideas,
  but its abstract's experiments are on Qwen2.5-7B and mobile deployment, not our
  1.7B checkpoint or GTX 1070. [Abstract][paper-edge]

These directions belong on a research watchlist or in specialized tools. None
justifies silently replacing Alt's model/engine or promising unlimited reasoning.

## 9. Failure evidence matters as much as success evidence

General AgentBench's 2026 abstract and current README report a **context ceiling**
for sequential scaling and a **verification gap** for parallel scaling: candidates
may contain more correct answers, but the agent cannot reliably select them.
Its evaluated agents are largely large/hosted models, so this is a warning to test
the mechanism, not a direct Spark measurement. [Abstract][paper-general]
[repository][generalbench]

The self-correction study similarly concerns intrinsic correction without outside
feedback. Concrete observations can enable useful repair, while asking the same
model whether it agrees with itself is a weaker signal. [Abstract][paper-correction]

Classify the actual bottleneck before changing a profile:

| Observed failure | First intervention to test |
|---|---|
| Correct proposed repair cannot be applied | Simpler editing protocol from prior harness research |
| Wrong/missing API fact or version | Relevant documentation and executable observation |
| Invalid tool format or repeated argument mistake | Native parser/schema qualification and interface-specific training |
| Valid edit with wrong algorithm/boundary behavior | Bounded reasoning, counterexamples, candidate search or weights training |
| Long output or exhausted deadline with no useful action | Action reservation, compact context and measured adaptive effort |
| Several candidates pass weak checks but remain wrong | Stronger independent verification, not more self-votes |

These categories can overlap. An improvement claim needs independently evaluated
behavior, accurate completion claims and a resource tradeoff useful to the PC's
owner. More convincing output is not itself a reasoning gain.

## 10. Reuse and the development decision

| Component | Inspected license | Recommended role |
|---|---|---|
| S* / SkyThought | Apache-2.0 | Study execution-grounded selection; adapt a small serial experiment |
| DART | MIT | Adaptive-effort reference after fixing/integrating the client contract |
| Plan-and-Budget | MIT | Decomposition/budgeting reference; qualify small-model transfer |
| OptiLLM | Apache-2.0 | Optional controlled proxy comparison; inspect each strategy independently |
| ThinkBooster | MIT | Search/scorer experimentation reference, with dependencies and total cost recorded |
| SOD / Open-R1 / veRL / rLLM | Apache-2.0 | Offline training infrastructure, separately hardware-qualified |
| Open-RS / Tree of Thoughts / PRM800K | MIT | Training/verifier/search references; do not copy unreviewed reward semantics |
| VibeThinker code/model cards | MIT | Reasoning-specialist reference; derivative and tool training need separate qualification |
| SmolLM3 model card | Apache-2.0 | Particularly relevant general tool-capable reasoning candidate |
| LFM2.5 model | Custom LFM Open License v1.0 | Optional candidate; commercial-use threshold terms need consideration |
| Nemotron research reasoning 1.5B card | CC-BY-NC-4.0 | Research reference, not a default unrestricted commercial weight bundle |
| Heretic code | AGPL-3.0 | Existing derivative tooling reference; code license differs from output model terms |
| Open-AgentRL 3K/30K dataset cards | No dataset grant established in inspected cards | Provenance/license research before reuse; code license does not settle it |

The inspected LFM license defines a $10M annual-revenue threshold and conditions
commercial use on not exceeding it. Model, dataset, framework and dependency
licenses are distinct; the table records source terms, not complete distribution
clearance. Original notices and applicable upstream terms must be retained.
[LFM license][lfm-license]

The next development pass should build **bounded native reasoning plus short
evidence-driven decisions**, followed by **two or three checked candidates under
a shared budget**. Qualify a tool-capable reasoning derivative alongside the
existing model. Once the interface and verifiers work, build a reviewed trajectory
corpus and try concise/mixed-length distillation; consider SOD/RL only when the
simpler weights experiment earns the added cost.

The [reasoning experiment proposal](2026-10-07-reasoning-experiments.md) defines
eight work orders and distinguishes inference improvements, model substitution,
training gains and added budget. This is a concrete route to a better local
system, with success measured rather than inferred from a model-size headline.

[r1-pdf]: https://github.com/deepseek-ai/DeepSeek-R1/blob/0cf78561f1d51c84a21b2190626b21116d5c68bb/DeepSeek_R1.pdf
[deepscaler-card]: https://huggingface.co/agentica-org/DeepScaleR-1.5B-Preview/blob/e3f524ce413a296b4d388e7560dd5c82c1c56725/README.md
[paper-openrs]: https://huggingface.co/papers/2503.16219
[openrs-config]: https://github.com/knoveleng/open-rs/blob/f75524b77bb759a6551dd644f145758b9668ef88/recipes/grpo.yaml
[openrs-rewards]: https://github.com/knoveleng/open-rs/blob/f75524b77bb759a6551dd644f145758b9668ef88/src/open_r1/rewards.py
[vibe15-pdf]: https://github.com/WeiboAI/VibeThinker/blob/5158cc982d1d8dbe68f2447c95aa4b32c7769d21/VibeThinker-1.5B.pdf
[vibe3-pdf]: https://github.com/WeiboAI/VibeThinker/blob/5158cc982d1d8dbe68f2447c95aa4b32c7769d21/VibeThinker-3B.pdf
[sod]: https://github.com/YoungZ365/SOD/blob/110c4b8e843aee274d3cd648199569369ee2403e/README.md
[paper-rstar]: https://huggingface.co/papers/2501.04519
[spark-card]: https://huggingface.co/XHToken/Spark-X2.5-1.7B/blob/14d6e83c13c7add2b62a7c39b2131f4ed1cddcf8/README.md
[paper-ttc]: https://huggingface.co/papers/2408.03314
[dart]: https://github.com/js-lee-AI/DART/blob/8393f7abc1beb16394b72827e1cc131d2fff1f0f/README.md
[paper-dart]: https://huggingface.co/papers/2606.23181
[dart-client]: https://github.com/js-lee-AI/DART/blob/8393f7abc1beb16394b72827e1cc131d2fff1f0f/dart/model_client.py
[dart-budget]: https://github.com/js-lee-AI/DART/blob/8393f7abc1beb16394b72827e1cc131d2fff1f0f/dart/sc_budget_router.py
[dart-equivalence]: https://github.com/js-lee-AI/DART/blob/8393f7abc1beb16394b72827e1cc131d2fff1f0f/dart/answer_extraction.py
[paper-sstar]: https://huggingface.co/papers/2502.14382
[sstar]: https://github.com/NovaSky-AI/SkyThought/blob/0d190f11fd8e885bbe113aeccacba5ccde5b1102/skythought/test-time-scaling/README.md
[sstar-script]: https://github.com/NovaSky-AI/SkyThought/blob/0d190f11fd8e885bbe113aeccacba5ccde5b1102/skythought/test-time-scaling/scripts/final_gentest_notimeout_cached/qwen3b_n_16_debug_public3_select_4omini_cached.sh
[sstar-driver]: https://github.com/NovaSky-AI/SkyThought/blob/0d190f11fd8e885bbe113aeccacba5ccde5b1102/skythought/test-time-scaling/evaluate_multiprocess.py
[tot-code]: https://github.com/princeton-nlp/tree-of-thought-llm/blob/8050e67d0e3a0fddc424d7fa5801538722a4c4cc/src/tot/methods/bfs.py
[optillm]: https://github.com/algorithmicsuperintelligence/optillm/blob/e4ad19906578c9c472a3f8fd04d218db89a31c8c/README.md
[optillm-mcts]: https://github.com/algorithmicsuperintelligence/optillm/blob/e4ad19906578c9c472a3f8fd04d218db89a31c8c/optillm/mcts.py
[thinkbooster]: https://github.com/IINemo/thinkbooster/blob/561489b1e037722b4210189d9ab09a75a9fc8159/README.md
[paper-ltm]: https://huggingface.co/papers/2205.10625
[paper-react]: https://huggingface.co/papers/2210.03629
[paper-tool-reasoning]: https://huggingface.co/papers/2507.05065
[planbudget]: https://github.com/junhongmit/P-and-B/blob/2ca2d90c6d3d630ec2b11cc22e0f56751f7f2b7e/README.md
[paper-planbudget]: https://huggingface.co/papers/2505.16122
[paper-flyby]: https://huggingface.co/papers/2609.34327
[smollm-blog]: https://huggingface.co/blog/smollm3
[paper-gap]: https://huggingface.co/papers/2502.12143
[paper-valley]: https://huggingface.co/papers/2506.07712
[paper-code-valley]: https://huggingface.co/papers/2510.06101
[data3k]: https://huggingface.co/datasets/Gen-Verse/Open-AgentRL-SFT-3K/blob/29f7c779329730068f425ef9995f50440ff9a3f2/README.md
[data30k]: https://huggingface.co/datasets/Gen-Verse/Open-AgentRL-30K/blob/660eca7324319cd69a57738228a5e8e336f3bb40/README.md
[openr1]: https://github.com/huggingface/open-r1/blob/5b6ff22b3fb7aa069c54866e517f39dfc3160e09/README.md
[verl]: https://github.com/verl-project/verl/blob/8718ca30a3f002f93b7c4fd99b9b2506718681bc/README.md
[rllm]: https://github.com/rllm-org/rllm/blob/3b40c37cf6a262cf4d28cc987ebe4f4cf797956c/README.md
[paper-rl-limit]: https://huggingface.co/papers/2504.13837
[paper-prorl]: https://huggingface.co/papers/2505.24864
[smollm-derivative]: https://huggingface.co/mradermacher/Huihui-SmolLM3-3B-abliterated-GGUF/blob/a22d8c90fba43ece78e78aeeed904a10148b7fb0/README.md
[deepscaler-derivative]: https://huggingface.co/mradermacher/DeepScaleR-1.5B-Preview-abliterated-GGUF/blob/7221c624325913cd66a61ffabafa89362a8ba7c3/README.md
[vibe15-derivative]: https://huggingface.co/mradermacher/Qwen2.5-1.5B-VibeThinker-heretic-uncensored-abliterated-GGUF/blob/37222026c5ba2d35efdd2dd7fc27cfc93cfb0342/README.md
[vibe3-fc]: https://huggingface.co/zkxxxx/VibeThinker-3B-heretic-fc/blob/1d247e8f362ef172911bfa7122436217acb91392/README.md
[lfm-derivative]: https://huggingface.co/mradermacher/Huihui-LFM2.5-1.2B-Thinking-abliterated-GGUF/blob/92b9a645ea84dfbdadef2fed197c77662769cceb/README.md
[nanbeige-derivative]: https://huggingface.co/megabytes/Nanbeige4.1-3B-heretic/blob/5ecf65019f075ece8fc6e0642500bb72b45895a8/README.md
[smollm-card]: https://huggingface.co/HuggingFaceTB/SmolLM3-3B/blob/a07cc9a04f16550a088caea529712d1d335b0ac1/README.md
[vibe3-card]: https://huggingface.co/WeiboAI/VibeThinker-3B/blob/77bd2cced09193c8b9a59a32bd8577bbd1f3e01c/README.md
[lfm-card]: https://huggingface.co/LiquidAI/LFM2.5-1.2B-Thinking/blob/f313478934a7612d22991f752959d7a1a8756fec/README.md
[lfm-license]: https://huggingface.co/LiquidAI/LFM2.5-1.2B-Thinking/blob/f313478934a7612d22991f752959d7a1a8756fec/LICENSE
[nanbeige-card]: https://huggingface.co/Nanbeige/Nanbeige4.1-3B/blob/9c4555c37921c0982af8fffeee1e0a6e6b953c4f/README.md
[phi-card]: https://huggingface.co/microsoft/Phi-4-mini-reasoning/blob/0e3b1e2d02ee478a3743abe3f629e9c0cb722e0a/README.md
[mimo-card]: https://huggingface.co/XiaomiMiMo/MiMo-7B-RL/blob/6299b5a2c45daf0c429285c92b8e61a5bd011c0d/README.md
[tinyzero]: https://github.com/Jiayi-Pan/TinyZero/blob/95df88f2dcb05f33bd18da546531b52d0954c18b/README.md
[paper-s1]: https://huggingface.co/papers/2501.19393
[prm800k]: https://github.com/openai/prm800k/blob/7ecc794703b2877f63226f2477a49b34f9b25163/README.md
[paper-prm]: https://huggingface.co/papers/2305.20050
[coconut]: https://github.com/facebookresearch/coconut/blob/27273cb8cca4bb763c041a63b036d0c3b7cbbb48/README.md
[paper-coconut]: https://huggingface.co/papers/2412.06769
[trm]: https://github.com/SamsungSAILMontreal/TinyRecursiveModels/blob/c01103738605ba39d1430519b1ee0c62f4c707f8/README.md
[paper-trm]: https://huggingface.co/papers/2510.04871
[paper-edge]: https://huggingface.co/papers/2603.16867
[paper-general]: https://huggingface.co/papers/2602.18998
[generalbench]: https://github.com/cxcscmu/General-AgentBench/blob/35f5c027c31ddcb3366b28674c6cb2957460c0e2/README.md
[paper-correction]: https://huggingface.co/papers/2310.01798
