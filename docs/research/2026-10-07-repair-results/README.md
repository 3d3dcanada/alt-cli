# Repair reliability evidence

October 7, 2026. Follow-up to the [22 failed coding attempts](../2026-10-07-harness-delivery/README.md).
The [repair work orders](../../REPAIR_WORK_ORDERS.md) describe the implementation.
Actual source behavior, transport completion and application protocol checks are
separate outcomes. Every live checkpoint is an explicitly selected
uncensored/abliterated derivative; there is no model or hosted-teacher fallback.

## Development screens

`exploration/` retains six failed attempts, including raw native arguments, check
failures, resulting source, actual runtime timings when completed, and each exact
compiled build. They used different development builds and deadlines and are
diagnostic screens, not a matched improvement study.

The early `compact` label used line-array edits before scalar and line-array
profiles were separated. Inspect the actual captured tool schemas to identify
the offered interface. Required-handle omissions, malformed nested-array arguments,
empty skill discovery and CPU deadlines are retained. Literal escaped text and
incomplete calls were refused rather than converted into executable model prose.

## Matched and fresh qualification

The [matched MiMo 9B Q4 pilot](mimo-matched/summary.json) completed all eight cells.
Both arms passed **3/4** unchanged independent v5 checks. This pilot establishes
useful repairs, not a reliability advantage or a promoted preset.

| Interface | Family | Repeat | Original oracle | Seconds | Actual requests | Initial input tokens | Initial preparation seconds |
|---|---|---:|---|---:|---:|---:|---:|
| Compact text | Two-file Python | 1 | Passed | 302.14 | 9 | 2,091 | 117.42 |
| Original | Two-file Python | 1 | Failed | 474.63 | 12 | 3,375 | 192.59 |
| Original | Python feature | 1 | Passed | 532.01 | 10 | 3,341 | 196.11 |
| Compact text | Python feature | 1 | Passed | 247.25 | 6 | 2,066 | 118.04 |
| Compact text | Two-file Python | 2 | Passed | 330.07 | 10 | 2,089 | 121.45 |
| Original | Two-file Python | 2 | Passed | 478.12 | 9 | 3,372 | 199.67 |
| Compact text | Python feature | 2 | Failed | 579.41 | 12 | 2,064 | 131.45 |
| Original | Python feature | 2 | Passed | 537.60 | 10 | 3,342 | 217.99 |

The exact common limits were 8K context, 1,024 total output tokens per response
including up to 128 reasoning tokens, temperature zero, 8,192 shared generated
tokens, 12 actual requests and a 600-second task deadline. Both used host workflow,
the same frozen Alt executable, Goose 1.53.0, llama.cpp b11429, the same
`MiMo-V2.6-Distill-Qwen-9B-heretic.Q4_K_M.gguf`, two CPU threads and batch 128.
The [campaign identity and order](mimo-matched/campaign.json),
[compiled input manifest](mimo-matched/BUILD.json) and
[actual runtime properties](mimo-matched/runtime-properties.json) pin the build,
weights, template and evaluator. Only the explicit tool/context focus changed.

Initial compact input was about 38% smaller on these tasks. Its three passing
attempts averaged 293 seconds; the original's three passing attempts averaged
516 seconds. Both failed one attempt. These descriptive measurements come from
two small families and cannot establish arbitrary-repository speed or quality.
The longer allowance differs from the earlier 22 failures; those counts cannot
be used as a like-for-like before/after estimate.

The failed compact repeat removed a function definition and pursued a temporary
check-copy path. Subsequent changes explain missing extracted definitions,
distinguish parser observations from compiler checks and identify disposable
traceback paths without rewriting raw diagnostics or blocking intentional edits.
Fresh validation of that subsequent build is recorded separately below.

## Subsequent portable build

All nine MiMo attempts on the subsequent portable build completed. This build's
source-input identity is
`6c00a2957ed3a723ec29746fd41153b42414263050cc6985d948597f96d4d2b9`.
It retains the matched pilot's exact model, runtime, engine, 8K context,
1,024-token output, 128-token reasoning allocation and ten-minute deadline.
The explicit interface is compact scalar text with host workflow. No selected
skill or extra instruction draft supplied any reference repair.

| Group | Case | Repeat | Original oracle | Normal turn | Seconds |
|---|---|---:|---|---|---:|
| [Python](final/mimo-final-feature/summary.json) | Feature repair | 1 | Passed | Yes | 340.16 |
| Python | Feature repair | 2 | Passed | Yes | 251.69 |
| [Languages](final/mimo-final-languages/summary.json) | Rust feature | 1 | Failed | No | 611.47 |
| Languages | JavaScript configuration | 1 | Passed | Yes | 277.05 |
| [Held-out](final/mimo-final-sealed/summary.json) | Interval merging | 1 | Failed | No | 604.61 |
| Held-out | Streamed UTF-8 lines | 1 | Failed | No | 611.98 |
| Held-out | Cache expiry | 1 | Passed | Yes | 518.88 |
| Held-out | Percent decoding | 1 | Passed | Yes | 286.56 |
| [Actual TUI](final/mimo-final-tui/summary.json) | Two-file Python repair | 1 | Passed | Yes | 312.86 |

Six of nine attempts passed unchanged independent behavioral assertions. Each
passing source also passed Alt's pinned current-source verification. The four
held-out families were not used to select prompts, settings or training traces.
One attempt per held-out family is a bounded check, not a general reliability
estimate. Timeout reports include cancellation/shutdown and external-check
overhead around the selected task deadline.

The actual 80×24 TUI receipt records four approved permission requests, visible
**Response ready**, normal Ctrl+Q exit, restored terminal attributes and no forced
signal fallback. Its raw terminal stream and persisted events are retained.
The [README screenshot](../../screenshots/v6/native-model-repair-80x24.png) is
a rendering of those bytes; its adjacent receipt pins the original stream.

The [Josiefied 7B scalar trials](final/josiefied-final/summary.json) used the exact
pinned Q4 artifact with the same context, output, shared request/token allowance
and ten-minute CPU deadline, without a thinking-token setting for this
non-thinking checkpoint. Both attempts failed unchanged assertions: the feature
repair left an unterminated string literal (609.68 seconds), and the two-file
repair remained behaviorally incorrect (609.95 seconds). Both timed out without
a normal completed model turn. Native file calls alone did not qualify this
artifact for these repairs.

[Follow-up plans](final/followup-plans/) freeze separately scoped development
experiments. The Rust combination changes selected skill, output, reasoning and
deadline together; its outcome cannot attribute a gain to reasoning alone.
Spark/Qwen scalar trials address earlier line-array screens. The additional 7B
line-array trial follows the development string-literal failure. None changes
the four sealed attempts or supplies their results to a model or training trace.

Cloud runs do not establish GTX 1070 throughput, VRAM fit, novice usability or
arbitrary-repository reliability. No quality preset is automatically promoted.

## Other checkpoints and reasoning/context follow-ups

These development trials use the same B06 portable source identity as the nine
MiMo tasks above. Their settings were fixed before each trial. They are separate
from the matched pilot and held-out results.

| Collection | Explicit change | Original oracle | Normal turn | Seconds | Issued requests |
|---|---|---|---|---:|---:|
| [Spark 1.7B](final/spark-final-scalar/summary.json) | Required scalar edits, 8K, ten minutes | Failed | Yes | 98.97 | 3 |
| [Qwen 4B](final/qwen-final-scalar/summary.json) | Required scalar edits, 8K, non-thinking, ten minutes | Failed | Yes | 432.08 | 12 |
| [Josiefied 7B](final/josiefied-final-lines/summary.json) | Explicit line-array edits, 8K, non-thinking, ten minutes | Failed | No | 610.01 | 12 |
| [MiMo Rust, 8K](final/mimo-rust-followup/summary.json) | Rust procedure, 2,048 output including 512 reasoning, fifteen minutes | Failed | No | 730.62 | 10 |
| [MiMo Rust, 16K](final/mimo-rust-16k-followup/summary.json) | Same Rust combination, explicit larger native window | Failed | Yes | 909.12 | 12 |

Spark left the requested feature unimplemented. Qwen ended normally with incorrect
behavior. The 7B line-array trial left an invalid Python function body. A normal
turn is a transport outcome and cannot stand in for the unchanged source oracle.

The 8K Rust combination stopped after ten actual requests because accumulated
input exceeded the output reserve. The 16K follow-up avoided that rejection and
reached twelve requests, then Goose reported its maximum-action boundary. It
still failed the original behavior check and Alt's current-source verification.
Its sampled Alt/Goose/runtime peak resident memory was 10.41 GiB; GPU memory and
fit on the reference PC remain unmeasured. The measured wall time includes final
shutdown and external checks around the 900-second task allowance.

The [native reasoning observation](final/reasoning-observation.json) records
3,126 reasoning-content characters in the 8K Rust trial and 2,252 at 16K, compared
with none in the earlier 128-token TUI repair trial. These are characters in raw
streaming deltas, not reasoning-token counts. The larger allocation emitted native
reasoning content, which establishes that the runtime accepted the setting;
it is not evidence of stronger reasoning or correct source. No reasoning preset
or larger-context default is promoted from these failed attempts.

## Original-request continuation repair

B07 source identity is
`90503bb799467c04bbddc3f891be1dcf6be523d76fea00b78d5b1f4b70064cbe`.
It adds the complete stored request to compact context when the latest user
message differs, and uses both messages for relevant source retrieval. The
latest message takes precedence. First-turn requests are not duplicated. Scope
that cannot fit fails clearly rather than being silently clipped.

All 119 Rust tests, formatting and strict Clippy passed. The
[protocol receipts](continuation/RAW-SHA256.json) and
[regression record](continuation/regressions/summary.json) retain exact build and
helper identities. Real Goose with scripted
providers exercised CLI resume through both adapters and edit formats, plus two
turns in the persistent 80×24 TUI. Long Unicode requests remained complete. These
weight-free protocol checks are separate from the B06 model outcomes. The cloud
setup initially stopped at an older candidate fixture's `v1:`-only handle parser;
the corrected fixture accepts the actual short handle and the candidate checks
and remaining ten stream-recovery trials passed. The initial failure is retained.

The first separately planned
[real MiMo two-turn TUI trial](continuation/mimo-b07-continuation/summary.json)
uses this exact B07 portable binary and the unchanged Python feature oracle. It
failed in 484.04 seconds: the source was incorrect, the first turn ended at twelve
requests, and the follow-up was rejected by the already spent shared allowance.
Both prompts were sent, only one normal turn completed, and terminal restoration
succeeded without a forced signal. The model reached native edits, actual checks
and Full terminal commands; none of those alone established correct behavior.

The [separate 24-request plan](continuation/plan-24.json) changes only the shared
request cap, leaving context, output/reasoning, generated tokens, per-turn engine
cap, model and total deadline unchanged. That
[completed trial](continuation/mimo-b07-continuation-24/summary.json) passed the
unchanged source oracle and Alt's current-source verification in **443.39 seconds**,
using **10 actual requests**. Both requested turns completed normally, four
permission choices were approved, Response ready was visible, Ctrl+Q exited
normally and terminal attributes were restored without a forced signal.
The [forwarded-context receipt](continuation/mimo-b07-continuation-24/continuation-context.json)
pins four actual follow-up request packets containing the complete original goal
once in the latest user message. Historical messages are retained separately.

This is one passing continuation trial alongside the earlier failure. The passing
run used fewer than twelve requests, so it does not establish that increasing the
cap caused the repair. Fresh project paths and native handles also differ between
attempts. No B06 measurement is silently relabelled as a test of B07, and no failed
trial is removed from the record. General multi-turn reliability remains unqualified.

## Reproduce a CPU repair trial

Use an optimized executable, the exact runtime/engine and pinned uncensored
weights. Choose an unused output directory; existing attempts are never erased
to create a clean-looking rerun. For example:

```bash
python3 scripts/live_acceptance.py --suite v5 \
  --binary target/release/alt --engine /path/to/goose \
  --runtime /path/to/llama-server \
  --model /path/to/MiMo-V2.6-Distill-Qwen-9B-heretic.Q4_K_M.gguf \
  --sha256 00877ff79174b79f72c5ec704901fc958400a0b0d390fb61a068281618116dd1 \
  --uncensored --verification-plan --tool-profile compact --workflow host \
  --contexts 8192 --output-tokens 1024 --reasoning-tokens 128 --temperature 0 \
  --generated-tokens 8192 --requests 12 --timeout 600 --threads 2 --batch 128 \
  --cases python-feature,python-multifile --repeats 2 \
  --output /path/to/new-output-directory
```

Add `--interface tui --width 80 --height 24` for the actual PTY interface.
For a persistent two-turn trial, also add
`--followup-prompt 'Continue the task and check your result.'` and set the total
deadline explicitly. Both turns share one request/token allowance and deadline.
This controlled evaluator selects CPU settings in a fresh state folder. Use
[PC testing](../../PC_TESTING.md) to measure your chosen GPU/runtime configuration.

## Evidence verification

Exact-archive install/container/rollback receipts are tracked in
[`release-evidence/2026-10-07/`](https://github.com/3d3dcanada/alt-cli/tree/main/release-evidence/2026-10-07),
outside the package payload so saving a receipt does not change the archive it
measures. The previous harness artifact and the current beta both report version
0.6.0; their actual source/executable/archive hashes distinguish them. Recovery
uses real prior project state, including the schema-3 to schema-4 migration.

Each collection has `RAW-SHA256.json`. The final `ROOT-SHA256.json` covers all
retained files in this folder other than itself. Manifests pin bytes, not model
success claims. Model weights, executables and private state databases are kept
outside Git. Failed attempts are retained alongside passes.
