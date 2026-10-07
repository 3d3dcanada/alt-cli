# Implementation and verification — Alt 0.6 beta

This pass turns the readiness review into the [0.6 work orders](WORK_ORDERS_0.6.md).
It remains a beta. Application checks, model task success, physical hardware
qualification and actual beginner usability are separate results.

## Delivered behavior

- Available-action instructions now match the selected tool focus, access and
  exact configured check names. These instructions sit outside trimmed memory.
  A structured-evidence error marks a tool call failed even if its process exits 0.
- Home offers **Prepare project checks**. Suggestions respect declared JavaScript
  package managers/test scripts and pytest configuration. Asking to run an empty
  check list opens setup. Nothing silently expands the selected tools or access.
- **Try a practice project** creates a fresh example with four independently
  pinned Python behavioral checks. It demonstrates a real failing seed, repair,
  saved evidence, restart and undo. `alt practice` also creates an example.
- Practice is visible near the top of Home at 80×24. Missing Python is reported
  as a missing interpreter, with a recovery instruction; a missing structured
  report no longer replaces the original execution error.
- File creation/edit submission replaces a disposable background file listing,
  so a slow listing cannot block the requested edit. Checks, saves and downloads
  remain nonreplaceable. Failed replacement work preserves the form draft.
  The same rule covers ordinary Task status refreshes, so opening Task and
  immediately configuring/running a check cannot race the automatic refresh.
  Context, Saved conversations and Jobs apply the same rule to passive reads.
  Post-turn verification announcements still finish before replacement.
- Small terminals retain both verification and the changed filename at 60×18;
  practice remains visible on Home at 80×24. Practice instructions and option
  descriptions fit the narrow dialog instead of ending halfway through a sentence.
- Memory presents complete source excerpts, records omitted excerpts, retrieves
  explicitly named files and filters conversational filler. Token-budget trimming
  reassembles memory rather than cutting through an excerpt. Current evidence
  retains priority over historical notes; native context is not enlarged.
- Search snippets preserve identifiers and original brackets. SQLite highlight
  markup previously made `cedar_inventory_port` look like
  `[cedar]_[inventory]_[port]`; a real Qwen continuation misread it as source.
  Sentence-ending punctuation also no longer prevents a named file lookup.
  Snippets can omit source sections, so their heading explicitly directs the
  model to read files before editing.
- Interrupted downloads finish pending writes before reporting a resumable
  offset. Repeated real HTTP truncations exercise this boundary.
- The Goose 1.53 adapter recognizes its synthetic network-failure message and
  reports an interrupted turn instead of successful completion. Partial streamed
  content is drained into history before returning an error. Recognition is
  specific to the pinned engine's message shape; other engines need their own
  protocol qualification.
- Package updates back up the selected state before replacing an existing
  executable, preserve the previous executable and abort if backup cannot be
  created. The package includes the local PC test recorder. Integrity checks remain
  active under `PYTHONOPTIMIZE=1/2`; a corrupted package cannot bypass validation
  through Python optimization. The negative control reproduces the former bypass.
- Release tooling supports GitHub identity-signed provenance, exact package
  verification and a prior-CI-artifact upgrade gate without a long-lived signing
  secret. The existing private-key candidate workflow remains available.

## Verification record

The [evidence index](evidence/v6/README.md) separates application checks, live
model outcomes, historical campaigns and exact package identities.

| Measurement | Result and scope |
|---|---|
| Rust checks | 93 tests passed; formatting and strict Clippy passed |
| Terminal journeys | Home, task, workbench, verification and practice passed through real PTYs |
| Responsiveness under project lock | 30/30 final-package journeys passed across three sizes; input p95 41.5 ms and cancellation p95 61.2 ms in this cloud |
| Practice recovery | 120×40, 80×24 and 60×18; edit/check, restart, undo and missing-Python recovery passed |
| Goose adapter | Actual Goose 1.53 against OpenAI-compatible and Ollama protocol fixtures passed; fixtures contain no weights |
| Sustained stream recovery | 60 scenarios across 12 normal/disconnect/cancel/engine-crash/reconnect cycles passed; partial history and process cleanup checked |
| Incremental retrieval | 5,001 files: 14.659 s initial, 0.085 s unchanged, 0.093 s edit/delete; debug build on shared cloud CPU with warm filesystem cache |
| Live memory continuation | Corrected Spark and Qwen each returned the required values in 3/3 observations, including restarts, 200 distracting notes and an edited source value |
| PC recorder | All five checks passed on cloud CPU with uncensored Spark; nine qualification rows across 2K/4K/8K plus a native-tool probe |
| Package gate | All seven local checks passed, including actual installed TUI and prior-version recovery on Debian 11 and Ubuntu 24.04 |
| Emulated older CPUs | QEMU 10.0.13: 14 startup/practice/runtime probes passed across Nehalem, Penryn and qemu64; one eight-token uncensored generation probe, not physical throughput or full engine coverage |
| Dependency audit | No advisories or warning groups in the recorded audit |
| Oracle controls | All 24 broken seeds rejected and known repairs accepted; selected incomplete repairs and Python/Rust/JavaScript early-zero-exit controls rejected |

The initial memory observations were Spark 3/3 and Qwen 2/3. Qwen misread search
highlight markup as source and missed saved requirements. After preserving raw
source in snippets, both models recorded 3/3 on fresh runs. These are six narrow
continuation observations, not a statistical improvement claim or a larger native
context window. Manual review also found that all six Spark responses incorrectly
attributed saved/pinned values to the source file, which contained only the port.
Correct JSON values therefore do not establish accurate explanatory prose. The
[12-response claim review](evidence/v6/memory/claim-review.json) retains that result.
The live recorder tests runtime operation and one small tool
interaction, not general coding reliability.

Spark completed all 120 attempts: **7/120** passed the pinned behavioral checks,
including **6/100 development** and **1/20 held-out**. All original hashes and
matrix cells validated. There were 106 action-budget notices, one output-token
limit and two cancellations near the configured 240-second deadline. The other
117 responses had protocol `end_turn`, which does not establish task success.
Median attempt wall time was 133.54 seconds and p95 was 218.37 seconds on the
hosted CPUs; this includes the post-turn oracle. No physical-GPU latency follows.

All 120 concluding responses were manually reviewed against saved source/checks,
with tool traces consulted for claimed edits and commands. Two falsely claimed
completion or edits/tests that had not occurred. Twenty-four incomplete responses
had specifically noted unsupported explanations (such as blaming correct checks
or supposedly stale snapshots). These categories describe the primary conclusion
and selected contradictions, not an exhaustive factual-accuracy percentage for
every sentence. [Analysis](evidence/v6/spark-analysis.json) and
[manual assessments](evidence/v6/spark-claim-review.json) retain the per-attempt
reasoning and original report hashes. Syntax/parser feedback is distinct from an
executed behavioral test.

Post-collection replay of all 120 retained sources against their original oracle
bytes reproduced every recorded result. The source was unchanged in 36 attempts.
The trace contains 78 exact-text edit mismatches, 52 edits attempted before the
explicitly configured plan requirement, and 34 missing/stale read hashes. These
are event frequencies, not independent failure rates or a causal explanation.
They identify workflow friction worth testing in a separate controlled campaign;
they do not justify quietly changing the frozen configuration.

Qwen completed all 120 attempts: **16/120** passed, including **11/100 development**
and **5/20 held-out**. Its six-minute deadline canceled 86 attempts; 34 ended with
protocol `end_turn`. Median wall time was 365.26 seconds and p95 was 366.44 seconds,
including the post-turn oracle. Source was unchanged in 60 attempts. All original
hashes validated and all 120 source replays reproduced the recorded outcomes.
The [Qwen analysis](evidence/v6/qwen-analysis.json) and
[120 manual assessments](evidence/v6/qwen-claim-review.json) record five false
completion claims and 60 incomplete responses with specifically noted unsupported
explanations. The same review limits apply: these are primary concluding-status
categories and selected contradictions, not exhaustive sentence-level accuracy.

Six [post-hoc counterexamples](evidence/v6/supplemental-counterexamples.json)
expose coverage limits in passing repairs across three task families: Spark date
attempt 5 rejects a valid timezone timestamp with fractional seconds; Qwen
topological-sort attempt 3 drops an isolated node; Qwen deduplication attempts 1–4
drop valid hashable keys or merge distinct keys with the same hash. Every probe
rejects the retained model repair and accepts the frozen reference repair. The
original scores remain unchanged. Expand a separately versioned future oracle;
current passes are not proof of complete implementations. Future fixture wording
should also state the required dictionary return type for `python-config` explicitly.

Spark used a 240-second deadline with thinking disabled; Qwen used 360 seconds
with template-default thinking. Both used 8K context, two threads, Coding focus,
an explicit plan requirement and a 12-turn engine budget. Different checkpoints,
budgets and reasoning settings prevent a matched comparison. Earlier pilots and
interrupted campaigns remain separate. No model preset is promoted.

### Evaluation defects found during this pass

The earlier harness could accept a project that exited successfully before its
assertions ran. Oracle contract 4 now requires a per-attempt completion receipt;
the Rust driver additionally requires the named behavioral test to complete. Rust
checks build against the actual checked snapshot. The collector copies declared
assertion inputs, excluding generated compiler output that interrupted an earlier
export. These changes preserve task goals and behavioral assertions. They do not
claim adversarial containment for arbitrary code running with Full access.

Fresh complete matrices use this contract. Old contract-3 results cannot be used
as a matched baseline or pooled into acceptance. Original available artifacts,
failed CI diagnostics and cancellation reasons are retained, including outputs
from incomplete campaigns. Unuploaded files on canceled runners cannot be
reconstructed. A model's completion claim or `end_turn` does not establish a
passing behavioral result.

### Package provenance

The final local package gate covers archive SHA-256
`06635b30e58b91db0c3b7c86dbf9f0ce891328c22fabbdf66914a789e54c0946`.
Its compiled input fingerprint matches the final UI source. The local build has
a dirty marker because documentation/evidence were being prepared; its build
record preserves that fact. Earlier local gates and memory/recorder binaries
are labelled by their own identities rather than represented as this archive.
The full model matrices use the earlier frozen executable; subsequent source
changes are confined to passive TUI refreshes, their tests and practice wording. The
[boundary record](evidence/v6/model-code-boundary.json) lists those exact paths.
Headless/tool/model/runtime sources did not change.
Publication rebuilds from a clean tag, reruns the exact-package upgrade gate and
verifies a GitHub workflow-identity attestation before creating the release. The
published archive therefore has its own hash and attached gate report.

**Publication completed:** [v0.6.0-beta.1](https://github.com/3d3dcanada/alt-cli/releases/tag/v0.6.0-beta.1).
The clean-tag package passed all seven release gates. Main/tag CI also passed.
The actual public download was independently checked against its GitHub asset
digest, checksum, attested workflow/tag/source commit, then installed into a fresh
temporary prefix. [The publication receipt](evidence/v6/published-beta/README.md)
records exact hashes and the validated signed-metadata transport used when the
cloud blocked Sigstore's CDN. The tagged archive is unchanged by these later receipts.

## Outside cloud acceptance

The [older CPU emulation record](evidence/v6/old-cpu/README.md) includes the
initial QEMU 8.2 infrastructure failures and successful QEMU 10.0.13 retry.
It does not qualify a physical machine.

The physical GTX 1070/8 GB VRAM/16 GB RAM Linux computer, other actual older PCs
and newer GPUs still need measurement. Follow [PC testing](PC_TESTING.md).
Actual uncoached beginner sessions require people; terminal automation does not
replace the [novice protocol](NOVICE_ACCEPTANCE.md). Private/gated model acquisition
requires applicable credentials, and real LM Studio remains a separate connection
qualification. Official LM Studio installer acquisition returned HTTP 403 both
in this cloud and on a separate GitHub-hosted runner; compatible-protocol fixture
passes are not represented as actual LM Studio inference. The cloud does not
supply those remaining observations.

See [0.5 evidence](IMPLEMENTATION_0.5.md) for the preceding snapshot. Its statements
about unpublished source and unrun remote CI describe that earlier point in time;
the 0.5 source and successful GitHub CI were subsequently published.
