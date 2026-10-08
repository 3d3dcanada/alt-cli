# Pre-change pilot evidence

Four declared baseline cells use released beta.3 and its exact frozen evaluator.
`baseline-manifest.json` was written before starting them. `baseline-summary.json`
links all four full reports, including the three failures. Each cell used the
same 8K CPU/two-thread/600-second condition and an explicitly uncensored artifact.
Cloud compilation ran concurrently; wall-time comparisons are descriptive only.
These historical tasks are development/validation, never final holdouts.

`retained-sha256.json` binds the retained ACP/provider text, independent oracle,
source snapshots and reports. `excluded-sha256.json` records files kept only in
the cloud workspace: executables, process configuration, SQLite, cache and build
state. Original attempt evidence manifests reference that fuller workspace and
are retained unchanged, so they are not complete manifests of this bounded copy.
No model weights, credentials or runtime processes are included here.

`run-baseline.py` is the original historical driver, retained as source evidence.
Do not rerun it in place: its fixed cloud paths describe the completed capture.
For a new experiment use `scripts/qualification_campaign.py` with a new output
directory, exact artifacts and declared budget as documented in
`docs/FINAL_PASS_QUALIFICATION.md`. Do not silently retry these four attempts.
