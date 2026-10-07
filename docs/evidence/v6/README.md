# Alt 0.6 evidence

Read [the implementation report](../../IMPLEMENTATION_0.6.md) for conclusions.
This directory preserves measured outcomes, including failures. Application
correctness, model task success, physical GPU support and human usability are
different acceptance gates. No model preset is promoted by these results.

## Application and recovery

| Evidence | Scope |
|---|---|
| [rust-tests.log](rust-tests.log), [formatting.log](formatting.log), [clippy.log](clippy.log) | Final 93-test Rust run, formatting and strict Clippy; formatting is silent on success |
| [tui-home.log](tui-home.log), [tui-task.log](tui-task.log), [tui-workbench.log](tui-workbench.log), [tui-verification.log](tui-verification.log) | Actual terminal journeys through application processes |
| [tui-pressure-final.json](tui-pressure-final.json) | 30/30 final-package keyboard/Unicode/cancel journeys under project-lock pressure; three sizes, observed input p95 41.5 ms and cancel p95 61.2 ms |
| [tui-practice.log](tui-practice.log), [tui-practice-package.log](tui-practice-package.log) | Practice failure/repair/check/restart/undo, three terminal sizes, unavailable Python recovery |
| [goose-openai.log](goose-openai.log), [goose-ollama.log](goose-ollama.log) | Real Goose 1.53 and Alt processes using deterministic HTTP fixtures; no model weights |
| [stream-recovery.json](stream-recovery.json), [stream-evidence/](stream-evidence/) | 60 normal/disconnect/cancel/engine-crash/reconnect scenarios and raw streams |
| [retrieval-5001.json](retrieval-5001.json) | Initial/unchanged/edit-delete indexing on a debug build, shared cloud CPU, warm filesystem cache |
| [old-cpu/](old-cpu/README.md) | 14 newer-emulator probes passed, including uncensored token generation; old-emulator failures retained; no physical PC or throughput claim |
| [dependency-audit.json](dependency-audit.json) | Recorded advisory database audit, no advisories/warning groups |
| [oracle-self-test-v4.json](oracle-self-test-v4.json), [oracle-collection-v4.log](oracle-collection-v4.log) | 24 fixture controls; actual snapshot checks, completion receipts and collection regression |
| [negative/](negative/) | Before-fix failures and blocked LM Studio acquisition, preserved as failures |
| [initial-checks/](initial-checks/) | Earlier candidate checks; not substituted for the final 93-test run |

The [screenshots](../../screenshots/v6/README.md) are rendered from recorded PTY
buffers. Their build identity and original text are retained. A manual repair
produced the displayed passing check; this is not an invented model success.

## Live uncensored models

Both checkpoints were explicitly selected and SHA-256 verified. No run switches
to a different model after failure. All these observations used CPU inference.

| Checkpoint | Pinned source | Artifact SHA-256 |
|---|---|---|
| Spark-X2.5-1.7B-Abliterated Q4_K_M | `darioooooo0o/Spark-X2.5-1.7B-Abliterated-GGUF` at `0193e8f9b55c7f8e9c58799694696a17d8f6c6c5` | `1e4d920aaff1248b68751e49b16dae6104c5f4d3924a86a290e24b31db08113c` |
| Qwen3-4B-Instruct-2507-heretic Q4_K_M | `mradermacher/Qwen3-4B-Instruct-2507-heretic-GGUF` at `7a992a7622cef2a223ca68744c898c4298f8ddd0` | `5d38e532cf33a5b56760bc7803aaabffb240d0604f1e389e4db148830c2bb747` |

[memory/](memory/) contains initial and corrected continuation probes. Each
cohort has three observations, raw prompts/streams/context, source snapshots,
model/runtime/build identity and hashes. Initial Spark scored 3/3 and initial
Qwen 2/3; each corrected cohort scored 3/3. These small recall probes do not
establish coding reliability or a statistical improvement. The separate
[claim review](memory/claim-review.json) found incorrect source attribution in all
six Spark responses despite correct JSON values. Qwen’s failed initial observation
misinterpreted search markup and dismissed available saved/pinned values.

[pc-kit-live/](pc-kit-live/) records the actual PC test recorder running the
selected Spark checkpoint on cloud CPU. It passed the build, hardware, connection,
model qualification and native-tool checks. Qualification includes nine rows
across 2K/4K/8K, actual generation, cancellation and owned-runtime restart.
[pc-kit-smoke/](pc-kit-smoke/) separately records the no-inference path. Neither
folder measures a physical GTX 1070.

The complete [Spark matrix](matrices/spark/summary.json) contains all 120 oracle-4
attempts, with **7 passing**: 6/100 development and 1/20 held-out. Raw sources,
streams, oracles and hashes are in [matrices/spark/](matrices/spark/).
[The analysis](spark-analysis.json) separates protocol endings, 106 action-budget
notices, latency, sampled memory and [120 manual prose assessments](spark-claim-review.json).
The review found two false completion claims and 24 incomplete responses with
specifically noted unsupported explanations. It assesses concluding task status
and selected contradicted claims; it is not exhaustive fact-checking of every
sentence or a statistical reliability estimate.
[Source replay](spark-source-replay.json) reproduced all 120 recorded outcomes
using the retained final sources and original oracle bytes; no new inference
or model retries were involved. [The replay script](source-replay.py) records
the exact cloud-side procedure. Tool error frequencies in the analysis are
terminal update counts, not per-task failure rates or causal estimates.
[The complete Qwen matrix](matrices/qwen/summary.json) contains all 120 attempts,
with **16 passing**: 11/100 development and 5/20 held-out. Its six-minute deadline
canceled 86 attempts. [Analysis](qwen-analysis.json) and
[120 manual assessments](qwen-claim-review.json) record five false completion
claims and 60 incomplete responses with specifically noted unsupported explanations.
[Source replay](qwen-source-replay.json) reproduced all 120 outcomes. Raw sources,
streams, oracles and hashes are retained in [matrices/qwen/](matrices/qwen/).
Spark used 240 seconds with thinking off; Qwen used 360 seconds with template
default thinking. These different configurations are not a matched comparison.

[Supplemental counterexamples](supplemental-counterexamples.json) identify six
passing attempts with behavior gaps across date parsing, dependency ordering and
deduplication. Each probe rejects the model repair and accepts the frozen reference
repair. These post-hoc observations do not alter the original scores or count as
new model attempts.

`scripts/analyze_matrix.py` binds supplied manual assessments to original report
hashes and never infers claim accuracy automatically. [Negative controls](analysis-controls.json)
reject a wrong model label, changed report identity and modified raw files,
including under `python -O`. Both complete campaigns retain every assigned attempt.

## Historical campaigns

[campaign-history.json](campaign-history.json) records interruptions and reasons.
[pilots/](pilots/) and [historical-campaigns/](historical-campaigns/) are separate
cohorts under the earlier execution contract. They are not acceptance results or
a matched baseline. Original available reports are preserved, including 116
reports recovered from Spark run 37549928064; the original completed summary
validated only 105/120 cells because one shard's collector failed.

Recovery branches were produced by GitHub runners because this cloud could not
download Actions' signed artifact URLs. `RECOVERED_FROM.json` identifies each
branch/run/commit. Missing files from canceled runners remain unavailable.
Generated compiler files were excluded from repository copies with their hashes
and reasons in `recovery.json` or `OMITTED-GENERATED.json`. Original manifests
are retained; omitted generated files are not rewritten as measured source.

## Packages and identity

The local package gate progressed through seven packaging snapshots:

| Snapshot | Gate | Archive SHA-256 |
|---|---|---|
| Initial 0.6 | [local-package-gate.json](local-package-gate.json) | `e466b92287371cf07db46306980ec3976315a9f0b14efdee5dfe719c96dd85b9` |
| Raw-source snippet fix | [local-package-gate-final-core.json](local-package-gate-final-core.json) | `319179372c560cd67a99968e51998f7cd91676fd3bb2a770c3bd5d53cca79e55` |
| File-listing/compact UI fixes | [local-package-gate-final-ui.json](local-package-gate-final-ui.json) | `971b49b2bba7808d473ae9ff5cbe2b90bf81ba7080346e04dab910338276944a` |
| Task refresh fix | [local-package-gate-task-refresh.json](local-package-gate-task-refresh.json) | `83ef9a976f81b2e9c14ec4c69f5aaa787e2a9edc5e5cc7e9bcd89cfe38b256c0` |
| Passive refresh fixes | [local-package-gate-passive-refresh.json](local-package-gate-passive-refresh.json) | `10a824d43373dc101aebff8d38b935c351b657d651497ad54ca4ae4fa1771090` |
| Installer optimization fix | [local-package-gate-installer-fix.json](local-package-gate-installer-fix.json) | `09c9530f8c457a95438d72e4c71a1e044d8f1d6066e686eddf0c9cc89b052917` |
| Complete narrow-terminal practice text | [local-package-gate-final.json](local-package-gate-final.json) | `06635b30e58b91db0c3b7c86dbf9f0ce891328c22fabbdf66914a789e54c0946` |

Matching `local-portable-build*.json` files preserve compiled inputs, commit and
dirty state. The local artifacts were unsigned and unpublished when measured.
Their seven checks cover archive integrity, repeated installation, upgrade,
rollback/re-upgrade, restored prior state, official SBOM schema and actual
installed terminal work on Debian 11/Ubuntu 24.04. Final installer checks also
reject corrupted, unlisted and linked payloads with `PYTHONOPTIMIZE=0/1/2`. The
[pre-fix control](negative/installer-optimization-before.json) and
[failing regression](negative/installer-optimization-regression-before.log) preserve
the bypass that prompted this fix. [legacy-upgrade.log](legacy-upgrade.log)
also checks 0.4.1 state, whose older verification receipts require fresh checks.

The published package is rebuilt from a clean tag and has its own attached
release-gate report, checksum and GitHub provenance bundle. Those external
reports refer to the exact distributed archive; putting that archive's own hash
inside itself would be circular. Model weights, application executables and
private state databases are excluded from this evidence directory.

[model-code-boundary.json](model-code-boundary.json) records the later TUI-only
source changes relative to the frozen model campaign binary. The measured model
executable is not described as the final packaged binary.

`MANIFEST.json` lists SHA-256 hashes for retained files in this directory (except
itself). Per-cohort raw manifests provide the original measurement boundaries.
