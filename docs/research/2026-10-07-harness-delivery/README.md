# Harness execution evidence

October 7, 2026. This folder accompanies the [delivery record](../../SMALL_MODEL_DELIVERY.md)
and [user guide](../../SMALL_MODEL_USAGE.md). Raw failures are retained alongside
passes. Nothing here qualifies a GTX 1070 or establishes an unlimited model.
The [folder manifest](ROOT-SHA256.json) hashes every retained file except itself;
attempt manifests additionally preserve the original execution identities.

## Actual uncensored model runs

[Model pins](model-pins.json) identify the exact repositories, revisions, files,
bytes and SHA256. Native reports contain the actual runtime/library/template
identity, settings, nonce, parsed calls and host observation. The native folders
retain the request bodies and available raw streamed responses.

| Probe | Result and raw evidence |
|---|---|
| Spark 1.7B Q4, automatic | [Failed report](spark-native.json); [raw exchange](native/spark/native-b491efa5-be30-48ea-8f4a-abb8e6750d33/) |
| Josiefied 7B Q4, initial | [Timed-out report](7b-native.json); [retained request](native/7b/native-dc2d8ea2-bc0a-4bd2-93fc-1d70a5721798/). No response prefix existed in this older attempt. |
| Josiefied 7B Q4, output 512/temp 0 | [Repeated-call failure](7b-native-600.json); [raw exchange](native/7b/native-55b12161-14ba-4b0c-9378-57fdca78c2ce/) |
| Josiefied 7B Q4, required/serial/output 256 | [Length-limited failure](7b-native-single.json); [raw exchange](native/7b/native-8ee58a3d-5ab8-4d15-8938-b8173855af24/) |
| MiMo 9B Q4, original automatic echo | [Passed basic round trip](9b-native.json); [two actual exchanges](native/9b/native-dbcd08b7-9c0c-4845-9327-f07b3f58b67e/). The predictable echo does not prove host-result consumption. |
| MiMo 9B Q4, challenge v2 | [Passed fresh-result consumption](9b-native-v2.json); [two actual exchanges](native/9b-v2/native-5c88e548-1961-4d87-a837-8884d335c8c0/). Approximately 41 seconds including startup, 4K context, 512 output, 128 reasoning, temperature zero, two CPU threads. |

The final challenge validates the input call, then creates a fresh host value
absent from the first request. The final response must contain that returned
value. [Actual HTTP protocol fixtures](native-protocol.json) show that repeating
the known input fails and consuming the new observation passes. These fixtures
have no model weights. Legacy `host_result_used` in the original report is a
weaker assertion and is not used as proof of consumption. Reports retain the
probe's own tool-schema hash separately from the configured Alt schema.
The [live probe build](native-v2-probe-build.json) pins the executed source.
The final decoder subsequently tightened unsupported termination rejection;
the live exchange uses supported `tool_calls` and `stop` terminations, and final
regression tests cover rejected statuses.

7B elapsed time includes a slow development-build integrity scan in the earlier
attempts; it is not pure generation time. Later optimized SHA256 scans still
rehash the entire artifact. Disposable 7B/9B evaluation downloads were cleaned
after preserving evidence to leave space for packaging. The exact 9B artifact
was subsequently [downloaded and verified again](9b-redownload.json) in temporary
storage for challenge v2; model weights are not committed or bundled.

The [actual MiMo optimizer attempt](offline-optimizer-9b/) consumed development
feedback from the failed Spark configuration task. It produced malformed tool
markup, not usable instructions. Its original receipt and response remain
unchanged; [post-run review](offline-optimizer-9b/REVIEW.json) rejects the candidate.
The run predates the rejection validator; its zero CLI exit is not evidence of a
valid proposal. No instruction was registered, validated or activated. The
validator snapshot documents the subsequent fix, not the executed optimizer version.

## Matched workflow pilot

[Spark pilot](pilot/spark/summary.json) compares model-written and host plans on
four development families, one repeat per arm. Both arms use the same artifact,
8K context, output 1024, temperature zero, Full access, full tool schema, two CPU
threads, 8192 total generated tokens and eight requests. Each attempt has a
180-second inference deadline. All eight attempts failed the independent oracle.
Some returned native tools, some consumed their output in reasoning, and one
timed out. A successful transport or check claim did not repair the requested
behavior.

[Qwen pilot](pilot/qwen/summary.json) repeats the same four families and two arms
with its own exact 4B artifact and 240-second deadline. All eight attempts timed
out and failed independent checks. Some issued native file-reading calls before
an unfinished request; its reserved output remains charged. Across both models,
this workflow pilot passed 0/16 attempts.

Each cell retains its frozen evaluator/oracle source, original fixture, actual
final source in `report.json`, independent check outputs, ACP trace and raw
provider bodies/receipts. `evidence-sha256.json` is the original attempt manifest;
`RAW-SHA256.json` hashes the exported evidence. Input receipts demonstrate actual
owned full-chat/tool token accounting. Interrupted generation keeps its full
reservation and remains unknown rather than zero.

[Spark](spark-pilot-build.json) and [Qwen](qwen-pilot-build.json) pilot build manifests
identify the separately frozen binaries and their source-input hashes. Both
predate the final notice scrolling and analysis durability changes. Spark also
predates the final memory preview and inclusion of skills in build provenance.
Within each model, the binary is identical across both
arms. The [final build manifest](current-build.json) describes the delivered
code; the pilot does not qualify that complete final build.

This is a bounded pilot, not the planned repeated 96/480-attempt screen or
confirmation. No preset is promoted and no gain over historical v0.6 is claimed.
Builds and fixture checks shared the cloud CPU during portions of this pilot;
wall times are not an isolated inference latency benchmark. The final campaign
runner freezes its own evaluator bundle for every cell. These pilot cells retained
identical evaluator/oracle hashes while those files remained unchanged; the new
freeze mechanism was added afterward and is tested separately.

## Matched thinking-budget pilot

[Spark thinking pilot](pilot/spark-thinking/summary.json) compares server-default
thinking against a 128-token thinking budget on two development families, one
repeat per arm. Both arms keep host workflow, 8K context, 1024 total output,
temperature zero, two CPU threads, 8192 shared generated tokens, eight requests
and a 180-second deadline. The final allocation keeps action headroom as the
shared budget shrinks. All four attempts failed independent checks. On the
feature task the cap changed the completion from an output limit to an end turn,
but the source still failed: seven requests/634 generated tokens versus two
requests/1139 tokens. Formatting or shorter reasoning is not correctness.

The [thinking-pilot build](spark-thinking-build.json) includes the final shared
allocation fix and freezes its evaluator bundle. It predates only the native
probe's fresh-host-result challenge and probe-schema hash; the coding inference
path is unchanged. All 20 coding pilot attempts across these studies failed,
so no default preset, success trajectory or reasoning gain is claimed.

## MiMo 9B skill comparison

The [MiMo skill pilot](pilot/mimo-skill/summary.json) uses the same pinned 9B Q4
artifact that passed the native host-result challenge. It compares host workflow
alone against the active Python repair skill on one development repair family,
with 8K context, 1024 output, 128 reasoning, temperature zero, two CPU threads,
8192 shared generated tokens, eight requests and 300-second inference deadlines.
Both attempts timed out and failed the independent oracle: **0/2**. The skill arm
made native file-reading calls; the other arm additionally saved a model plan.
Neither completed a verified repair, and no executable-helper quality gain was
observed. Timed-out calls retain their full generation reservation.

The [executed build](mimo-skill-build.json) predates only the final native decoder
termination guard; it uses the same coding inference path as delivery. The runner
and oracles are frozen beside each attempt. Builds and package checks shared some
CPU time, so the timeouts do not establish hardware-independent model inability
or an isolated latency result. Together with the Spark/Qwen studies this delivery
retains **22 failed coding attempts**, separately from MiMo's native-tool pass.

The original summaries count only outer evaluator timeouts/exit 124 in their
`timeouts` column. Alt's internal deadline cancels a turn with exit 1 and an
observed `cancelled` stop reason, so that column alone misses those cases.
[Post-run status aggregation](CANCELLATION-SUMMARY.json) adds observed cancellations
without inferring their cause, retaining the original reports and summaries
unchanged. The final campaign runner supplies both counts; its regression checks
that a cancelled turn is visible without inventing a timeout.

## Real helpers and application fixtures

[Incremental retrieval measurements](retrieval-index.json) cover 5001 files,
an unchanged rescan and a file edit/deletion. Bytes read and elapsed time are
actual. RSS is a sample of the whole process, not peak/index-only memory.

[Skill helper receipts](skills-helpers.json) record real Python, Cargo, JavaScript,
configuration validation and Chromium execution against broken/fixed behavior.
[The captured browser image](skills-browser-checks.png) retains the actual result.
Dependency-review receipts retain actual pip/npm advisories and Semgrep results.
They establish helper
operation; they do not show that a language model applies the skill successfully.
The validation manifest and logs distinguish these checks from protocol fixtures,
which use real HTTP/Goose/PTYs but scripted responses with no model weights.

[Validation](VALIDATION.json) identifies the executed test counts, logs and local
package gate. The package is unpublished; the previously published beta archive
predates this work. Its installer, upgrade/rollback, integrity checks and the exact
packaged frontend are exercised on Debian 11 and Ubuntu 24.04. The installed
skills and training handoff are checked too. The final package receipt is added
after packaging, so the archive contains an earlier documentation snapshot.

The training tests verify capture/schema/split/loss-mask/resume/export/reward
boundaries. They do not produce an approved corpus, a useful adapter, an exported
GGUF, SOD/OPD training or an RL result. See the [training handoff](../../../training/README.md).

| Cloud check | Observed result |
|---|---|
| Final Rust suite, formatting, strict Clippy | 105 passed, zero failures; after all Rust changes |
| Training preparation/capture/encoding/resume/export/reward boundaries | 46 passed; no GPU training |
| Adapter/optimizer boundaries and campaign boundaries | 8 and 3 passed; frozen imports, missing-source rejection and truthful cancellation counts |
| V5 independent oracle self-check | 28 families; broken seeds, reference repairs, early exit, altered oracle and stale source checked |
| Real Goose provider protocols | OpenAI-compatible and Ollama fixtures passed; scripted responses, no weights |
| TUI flows | General/task/workbench/verification/practice checks passed; new controls passed at 120×40 and 80×24 |
| Executable skill helpers | Python, Rust, JavaScript, configuration, Chromium, pip/npm audit and Semgrep exercised |
| Locked Rust dependency audit | Zero vulnerabilities/warnings in the recorded advisory database |
| Exact local portable package | Seven release gates passed, including installed Debian 11/current Ubuntu, integrity and actual previous-package upgrade/rollback |
