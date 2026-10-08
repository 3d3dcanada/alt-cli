# Beta 3 release finalization

Beta 2 tag verification (run 37703575696, source 1a0e233) failed because a
child exited while `smoke_tui.py` read `/proc/<pid>/status`. Linux can raise
`ProcessLookupError` after the file opens, whereas the probe only caught
`FileNotFoundError`. All 123 Rust tests and strict Clippy had passed before
this test failure. The complete failed job log is preserved here, retrieved
through diagnostic run 37704588431 and its dedicated evidence branch.

Beta 2 publication run 37703575716 was cancelled before any release was
created. Its tag is retained unchanged. Beta 3 corrects both cleanup probes
through a shared helper and adds five regression tests. Disappeared processes
count as exited; live children, mismatched process identities and unexpected
read errors still receive the appropriate handling. No model prompt, tool,
engine, allowance, oracle or runtime source changed in this correction.

The clean beta 3 publication checks are attached to its GitHub release. Local
checks for the helper correction are recorded alongside this document. The
beta 2 development model records in `../pc-ready/` remain unchanged and retain
their original build hashes, settings and failed outcomes.

Publication now waits for successful full Verify CI on the exact pushed tag
and commit, including installed-package stress checks. Five additional gate
regressions reject unrelated/main runs, newer failures, non-success conclusions,
missing runs and deadline expiry. `beta2-gate-rejection.json` uses the real API
and proves the gate rejects the retained failed tag run. The release attaches
`tag-verification.json` alongside its package/provenance receipts.

`setup-result.json` and `setup.log` record the entire corrected setup completing
successfully: 123 Rust tests, strict format/Clippy, both adapters, twelve-turn
dirty-Git continuations, both compact formats, actual terminal journeys, the
PC recorder and ten streamed recovery attempts. These weight-free application
checks do not add to the earlier real-model success counts.
