# Evaluate the base before committing to a fork

**October 5, 2026. Status: proposed experiments; none executed.**

**Updated user requirements:** all live model/prototype tests use uncensored or abliterated checkpoints. The reference target is Linux with a GTX 1070 (8 GB VRAM) and 16 GB RAM; include CPU/older-computer operation. See the [current build plan](../BUILD_PLAN.md).

The research recommendation is Goose first, Stakpak as the challenger, and AIChat as a lightweight control. This plan is intended to change that recommendation if measurements favor another base. It evaluates the whole chain: CLI, agent loop, model, serving configuration, and tools.

## Questions to answer

1. Which existing Rust base completes the intended software-testing tasks with the least new infrastructure?
2. Can a 4B model select tools, supply valid arguments, use the results, and stop reliably?
3. Which verified uncensored derivative and quantization delivers the best task reliability on the reference machine?
4. Which failures come from the model, template/parser, provider adapter, or executor?
5. What hardware and context budget deliver usable task latency?

Do not evaluate all model/backend/framework combinations at once. Start with one baseline and two finalists, then expand where the results justify it.

## Phase 0: validate the protocol and runner

First use deterministic, harmless tools and controlled provider responses. This checks infrastructure independently of model quality. A model generating plausible prose is insufficient to establish that the integration works.

| Fixture | Expected evidence |
|---|---|
| Plain text response | Correct completion and clean termination |
| One valid call to a harmless tool | Exactly one execution and the correct result returned to the conversation |
| Tool arguments split across stream chunks | Arguments assembled once before execution |
| Unknown tool name | No process launched; explicit error |
| Missing required field or wrong argument type | Rejected before execution; useful corrective result |
| Malformed JSON followed by a repair | Bounded repair; no fabricated successful call |
| Parallel calls with distinct IDs | Correct ID/result association, or explicit unsupported behavior |
| Tool process times out or produces excessive output | Process tree terminated; truncated model-facing output; raw artifact/status preserved |
| Cancellation during inference or execution | Session terminates promptly and leaves no owned child process running |
| Stream disconnects after a tool was dispatched | No blind duplicate execution of a non-idempotent action |
| Missing tool result or context compression boundary | Valid remaining transcript with correct call/result pairing |
| A file/tool result contains fake instructions | It remains task data; the configured execution scope still holds |

A practical initial tool contract includes registered name, JSON Schema, side-effect category, argument validation, timeout, output quota, and cancellation behavior. Boundaries should be enforced by the runner, independently of the prompt or the model's alignment.

For MCP, repeat the important fixtures over the selected transport, including server disappearance and a changed tool list. A working `tools/list` response alone is not enough.

## Phase 1: compare the two CLI bases

Use one verified **uncensored/abliterated 3–4B model**, with a pinned artifact and explicit chat template. The Heretic-published Qwen3-4B-Instruct-2507 derivative is a provisional candidate, subject to direct file/license/compatibility checks. A Spark-X2.5 derivative can join the queue once verified. Begin with a Pascal-compatible backend for the GTX 1070 and a CPU path. Add the second backend only after the first works.

Keep these variables the same wherever the adapters allow it:

- Task wording and initial project/lab state.
- Tool inventory, descriptions, schemas, output format, and executable implementation.
- Model artifact, quantization, sampling parameters, context/output budgets, and template settings.
- Maximum turns, corrective retries, tool timeouts, and process scope.
- Evidence collection and independent success checks.

Record any setting that cannot be matched. An upstream default prompt is part of a product and can be tested separately, but should not be silently changed mid-comparison.

Use this **20-task pilot** before a broader benchmark:

| Group | Count | Representative tasks and verification |
|---|---:|---|
| Repository inspection | 4 | Locate a configuration problem, identify an unsafe pattern in a fixture, connect an error log to a source location, distinguish a documented setting from an actual value. Verify paths and evidence. |
| Test/build workflow | 4 | Run an existing test, identify an intentionally failing assertion, interpret a build error, inspect a dependency report. Check actual runner output and exit status. |
| Local application checks | 4 | Verify expected headers, a known authorization behavior, a validation error, and a documented endpoint against a disposable local fixture. Judge against the fixture's known behavior. |
| Tool and error recovery | 4 | Handle a missing executable, a timeout, large output, and a corrected malformed call. Verify recovery stays within budget. |
| Reporting and control | 4 | Produce evidence-backed findings, avoid inventing a result when data is absent, preserve a failed check, and honor cancellation/scope in an injected-instruction fixture. |

The application checks are confined to the supplied lab. The goal is assessing the assistant's correctness and tool handling, not selecting a more capable attack payload.

Run the same cases **three times** to expose instability. Two bases × one model × one backend × 20 tasks × three trials = **120 runs**. Repeating on the second backend produces another 120. This is a pilot for comparing failure patterns; it is not enough to make sweeping model-quality claims.

Run AIChat on a smaller subset as a control if the two finalists show similar outcomes. If all hosts fail the same simple tool task, inspect the model/template/server path before adding more orchestration code.

## Phase 2: measure small-model and derivative tradeoffs

Using the more promising CLI setup, compare two verified uncensored/abliterated checkpoints. Keep the selected backend fixed, then compare two supported quantizations/precisions when available. Two checkpoints × two artifact precisions × 20 tasks × three trials = **240 runs**. With only one verified model, start with the precision comparison and report the smaller scope.

A within-checkpoint precision comparison helps isolate quantization effects; differences between separate derivatives do not isolate the causal effect of abliteration. An unmodified-versus-modified live comparison is deferred unless the user changes the testing requirement. If equivalent artifacts are unavailable, report the confounder explicitly instead of claiming a controlled comparison.

Then test verified uncensored derivatives from other small families, including Spark-X2.5 1.7B/4B when suitable artifacts are available, and larger models on hardware that can accommodate them. Any additional interpreter/dispatcher model used in a live test must satisfy the same user requirement. A model family's original checkpoint is research material, not automatic permission to substitute it in live tests.

Compare native tools against a constrained-action fallback or Goose's experimental shim on cases where native tools fail. Count all interpreter calls and memory consumption. A second model should not become an invisible dependency of a supposedly small-model system.

## Metrics and evidence

| Metric | How to measure it |
|---|---|
| **Task success** | Independent verifier checks the final artifact or observed application behavior; do not use the agent's self-report alone |
| **Tool selection accuracy** | Intended operation compared with the selected registered tool |
| **Argument correctness** | Schema-valid calls and semantically correct targets/values, recorded separately |
| **Unsupported behavior** | Explicit unsupported API/model cases separated from ordinary model mistakes |
| **Recovery rate** | Known injected failures recovered within the same fixed budget |
| **Fabricated findings/results** | Claims without matching tool execution or evidence; inspect false positives and false negatives |
| **Refusal behavior** | Recorded separately from malformed output, incorrect execution, or an honest lack of evidence |
| **Latency** | Cold and warm runs separately; time to first useful action and total successful-task duration |
| **Resource use** | CLI memory, inference-server RAM/VRAM, tool subprocesses, CPU/GPU utilization, and prompt/context size |
| **Cost in model calls** | Turns, prompt/output tokens where available, repair calls, shim calls, and any escalation |
| **Operator effort** | Manual corrections and approval interactions per completed task |
| **Integration effort** | Necessary source changes, new runtime dependencies, build complexity, and upstream-maintenance burden |

Store task ID, trial ID, timestamps, agent/framework commit, server version, model revision/hash, quantization, template hash, sampling parameters, hardware, tool versions, and configured budgets with every result. Keep tool events and raw artifacts alongside the independently determined result.

Classify outcomes as pass, fail, unsupported, infrastructure error, cancelled, or not run. Record any skipped cases with their reason. Report both aggregate results and the important per-task failures; a single percentage can hide a broken critical workflow.

## Acceptance criteria for the first implementation

These are proposed engineering gates, not reported results:

1. Every protocol/executor fixture has its documented expected outcome. Invalid or unknown calls never launch a process.
2. The selected local model can complete representative inspect → tool → result → report tasks, repeatedly, with verifiable evidence.
3. Cancellation, process limits, and tool-result accounting work through the chosen adapter.
4. The chosen workflow stays within the actual target hardware budget, without hidden cloud calls or an unreported second model.
5. There is a reproducible profile: exact model artifact, template, server, tools, and agent version are recorded.
6. The code and artifact licenses for the proposed distribution have been checked at the versions actually used.

Do not assign a universal minimum task-success percentage before judging task difficulty. For the pilot, compare paired outcomes and require all critical process-control fixtures to pass. Document which real tasks remain unreliable and constrain the initial product promises accordingly.

## Reuse existing evaluation work

[Berkeley Function Calling Leaderboard (BFCL)][bfcl] supplies function-calling and multi-turn evaluation categories and supports self-hosted endpoints. Use relevant cases to supplement the local tool-contract checks; its public leaderboard is not a measurement of our quantized derivative in our CLI.

[Terminal-Bench][terminal-bench] evaluates end-to-end terminal tasks. Its current README directs new users to [Harbor][harbor], the framework for Terminal-Bench 2.0. Consider Harbor for containerized task verification once the small pilot works. Select a representative subset first instead of immediately running an expensive full suite.

Benchmark definitions, dataset permissions, exact versions, required services, and verifier behavior need review before adoption. Fix the fixtures and success checks before observing candidate results, and preserve failed runs.

## Implementation sequence after research

1. Complete direct Hub artifact/license inspection for the uncensored model and inspect the GTX 1070 host's available inference stack.
2. Pin releases of Goose and Stakpak and build their relevant CLI targets.
3. Start one inference server and verify a complete harmless native-tool round trip.
4. Add a small shared tool pack and run the protocol fixtures and 20-task pilot.
5. Select the base on the recorded results, then create the downstream distribution or component integration.
6. Add derivative models and additional backends incrementally, retaining a compatibility matrix.

The research phase has produced source-backed recommendations and a testable decision process. Builds, model downloads, live inference, benchmarks, and the application implementation remain subsequent work.

[bfcl]: https://github.com/ShishirPatil/gorilla/blob/6ea57973c7a6097fd7c5915698c54c17c5b1b6c8/berkeley-function-call-leaderboard/README.md
[terminal-bench]: https://github.com/laude-institute/terminal-bench/blob/d28711d0da2675d0bb1d56de45ae5df6082438a3/README.md
[harbor]: https://github.com/laude-institute/harbor/blob/5c8eda619a9dfecba39227d41ff0b3b85c5028f8/README.md
