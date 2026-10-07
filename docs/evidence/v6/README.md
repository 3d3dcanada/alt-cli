# Alt 0.6 evidence

Read [the implementation report](../../IMPLEMENTATION_0.6.md) for conclusions.
This directory preserves measured outcomes, including failures. Application
correctness, model task success, physical GPU support and human usability are
different acceptance gates. No model preset is promoted by these results.

## Application and recovery

| Evidence | Scope |
|---|---|
| [rust-tests.log](rust-tests.log), [formatting.log](formatting.log), [clippy.log](clippy.log) | Final 91-test Rust run, formatting and strict Clippy; formatting is silent on success |
| [tui-home.log](tui-home.log), [tui-task.log](tui-task.log), [tui-workbench.log](tui-workbench.log), [tui-verification.log](tui-verification.log) | Actual terminal journeys through application processes |
| [tui-pressure.json](tui-pressure.json) | 30/30 final-package keyboard/Unicode/cancel journeys under project-lock pressure; three sizes, observed input p95 40.2 ms and cancel p95 61.2 ms |
| [tui-practice.log](tui-practice.log), [tui-practice-package.log](tui-practice-package.log) | Practice failure/repair/check/restart/undo, three terminal sizes, unavailable Python recovery |
| [goose-openai.log](goose-openai.log), [goose-ollama.log](goose-ollama.log) | Real Goose 1.53 and Alt processes using deterministic HTTP fixtures; no model weights |
| [stream-recovery.json](stream-recovery.json), [stream-evidence/](stream-evidence/) | 60 normal/disconnect/cancel/engine-crash/reconnect scenarios and raw streams |
| [retrieval-5001.json](retrieval-5001.json) | Initial/unchanged/edit-delete indexing on a debug build, shared cloud CPU, warm filesystem cache |
| [dependency-audit.json](dependency-audit.json) | Recorded advisory database audit, no advisories/warning groups |
| [oracle-self-test-v4.json](oracle-self-test-v4.json), [oracle-collection-v4.log](oracle-collection-v4.log) | 24 fixture controls; actual snapshot checks, completion receipts and collection regression |
| [negative/](negative/) | Before-fix failures and blocked LM Studio acquisition, preserved as failures |
| [initial-checks/](initial-checks/) | Earlier candidate checks; not substituted for the final 91-test run |

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

Full coding matrices are being collected under oracle contract 4. The final
report will record 100 development and 20 held-out attempts per model, or
explicitly identify missing cells. A completed measurement may still contain
zero successful tasks. Final prose must be checked against actual behavior.

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

The local package gate progressed through three source snapshots:

| Snapshot | Gate | Archive SHA-256 |
|---|---|---|
| Initial 0.6 | [local-package-gate.json](local-package-gate.json) | `e466b92287371cf07db46306980ec3976315a9f0b14efdee5dfe719c96dd85b9` |
| Raw-source snippet fix | [local-package-gate-final-core.json](local-package-gate-final-core.json) | `319179372c560cd67a99968e51998f7cd91676fd3bb2a770c3bd5d53cca79e55` |
| Final UI fixes | [local-package-gate-final-ui.json](local-package-gate-final-ui.json) | `971b49b2bba7808d473ae9ff5cbe2b90bf81ba7080346e04dab910338276944a` |

Matching `local-portable-build*.json` files preserve compiled inputs, commit and
dirty state. The local artifacts were unsigned and unpublished when measured.
Their seven checks cover archive integrity, repeated installation, upgrade,
rollback/re-upgrade, restored prior state, official SBOM schema and actual
installed terminal work on Debian 11/Ubuntu 24.04. [legacy-upgrade.log](legacy-upgrade.log)
also checks 0.4.1 state, whose older verification receipts require fresh checks.

The published package is rebuilt from a clean tag and has its own attached
release-gate report, checksum and GitHub provenance bundle. Those external
reports refer to the exact distributed archive; putting that archive's own hash
inside itself would be circular. Model weights, application executables and
private state databases are excluded from this evidence directory.

`MANIFEST.json` lists SHA-256 hashes for retained files in this directory (except
itself). Per-cohort raw manifests provide the original measurement boundaries.
