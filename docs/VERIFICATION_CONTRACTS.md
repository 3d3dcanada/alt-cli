# Check purposes and independent evidence

Alt separates three questions: did a command succeed, did every required check
pass on the current project, and did the selected independent behavioral assertions
pass? A build, linter, service health request or custom command cannot establish
behavioral acceptance by exit code alone. Unknown coverage stays unknown.

In Task, choose **Configure checks → Set check purpose and evidence**. Select a
saved check, its purpose, and (for tests) JSON, JUnit XML or TAP. Choose a relative
report path. A previous report is removed from the disposable source copy before
execution. Missing, malformed, incomplete, contradictory, failing and zero-test
reports cannot pass a Tests contract. Inspect raw output and the structured result
in Task; project exports include the original structured report and its hash.

For independent acceptance, select an assertion file outside the project. The
wizard pins its SHA256 and runs a separately staged copy through Python, Node or
shell. The assertion must inspect the current directory, which is the disposable
source copy, and write its result to `ALT_CHECK_REPORT`. Do not hard-code the
original project path: that would test a different copy. A changed assertion must
be reviewed and pinned again. User-selected assertions establish only what they
actually test; pinning does not make a weak assertion comprehensive.

Equivalent CLI setup, from the project folder:

```bash
alt task configure-check acceptance --kind tests --format json \
  --report .alt-check-results.json --assertion /trusted/acceptance.py \
  -- python3 '{assertion}'
alt task require requested-behavior --check acceptance \
  --description 'Describe the actual behavior this assertion exercises'
alt task verify --run
```

`contract` replaces the already configured script path with the separately staged
assertion. It rejects an unrelated command. `{assertion}` and `{report}` are
standalone argument placeholders, not shell substitutions.

The JSON schema is intentionally small and case based:

```json
{"schema":1,"complete":true,"tests":[
  {"name":"empty input returns an empty list","status":"passed"},
  {"name":"invalid input raises ValueError","status":"failed"}
]}
```

Names must be distinct; statuses are `passed`, `failed`, or `skipped`. Totals alone
are not evidence. JUnit requires explicit `testcase` entries and consistent
suite counts where present; DTDs/entities are rejected. TAP supports explicit
numbered `ok`/`not ok` results and exactly one complete `1..N` plan. Nested subtests,
bailouts, missing/duplicate results and unsupported records fail closed. Reports
are UTF-8, at most 8 MiB and 100,000 cases. Skips/TODOs do not count as executed
passing tests.

Evidence becomes stale after source/mode, command, contract, pinned assertion,
verifier version, relevant lockfile, executable or dependency inventory changes.
Installed `.venv`, `venv` and `node_modules` inventories include inode, size,
mode and nanosecond modification/change times. Linked package targets are followed
with cycle protection and bounded traversal. This is a local freshness mechanism,
not a cryptographic attestation of every system library. The environment may change
outside those tracked inputs; rerun checks after system updates.

Current execution records also hash the effective environment (without retaining
raw secret values) and declared external files. Add external inputs with
`--input-file PATH` and newly generated outputs with `--generated-output GLOB`.
Existing source inputs cannot be excluded as generated output. Source content,
mode and file identity are observed before and after execution; a detected change,
including mutate/test/restore identity changes, prevents certification of the
original source. The retained patch and manifests explain the change. These are
observations, not proof of immutable execution: records explicitly retain
`immutable_inputs: false`.

Each command reserves receipt capacity and creates raw streams before execution.
Completion is reconciled idempotently after project-lock contention or restart;
reconciliation does not rerun the command. Raw stdout and stderr retain a bounded
4 MiB tail each with offsets and hashes. Use Task's raw-output viewer or
`alt task logs RESULT_ID --stream stderr --limit 16384`. The UI preview is bounded
separately. A log read with `--offset` makes truncation/available ranges explicit.

Guided checks use Bubblewrap with a read-only assertion mount. Full access retains
normal host permissions, including arbitrary terminal commands and network use.
Its assertions are pinned and checked for changes, but Full access is not a hostile
code isolation boundary. Model text, successful tool calls and edited project tests
do not automatically become independent behavioral acceptance.

Configured checks and candidate exploration use bounded source snapshots: at most
4,096 files, 64 MiB total and 1 MiB per file. Generated/dependency directories and
sensitive paths are excluded by the project's snapshot rules. A limit failure is
reported; it cannot become a passing check. Native text edits have the same 1 MiB
file limit. For larger projects, choose a smaller project root or use the Full
terminal to run the project's normal commands. Terminal results do not automatically
satisfy the independently pinned check contract.

Project state schema is now version 5. Before upgrading an existing installation,
make a state backup. Executable rollback does not downgrade SQLite or settings:
restore the matching older backup into a new data directory and retain the newer
state for recovery. Opening a newer database is rejected before schema changes.
