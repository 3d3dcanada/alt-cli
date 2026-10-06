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
alt task configure-check acceptance -- python3 /trusted/acceptance.py
alt task contract acceptance --kind tests --format json \
  --report .alt-check-results.json --assertion /trusted/acceptance.py
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

Guided checks use Bubblewrap with a read-only assertion mount. Full access retains
normal host permissions, including arbitrary terminal commands and network use.
Its assertions are pinned and checked for changes, but Full access is not a hostile
code isolation boundary. Model text, successful tool calls and edited project tests
do not automatically become independent behavioral acceptance.

Project state schema is now version 3. Before upgrading an existing installation,
make a state backup. Executable rollback does not downgrade SQLite or settings:
restore the matching older backup into a new data directory and retain the newer
state for recovery. Opening a newer database is rejected before schema changes.
