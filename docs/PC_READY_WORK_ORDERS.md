# PC beta completion work orders

These work orders implement the five recommendations from the final cloud review.
They preserve the selected model, provider, access policy, original assertions and
failed evidence. Cloud CPU measurements do not establish GTX 1070 performance.

| Order | Deliverable | Acceptance |
| --- | --- | --- |
| P01 | Current downloadable beta, release notes and consistent installation instructions | Exact package passes integrity, installed application, upgrade/rollback and provenance gates before publication; README and PC guide point to its tag |
| P02 | One-command disposable PC repair and recovery recorder | Failing seed, actual model edit, independent behavior check, second turn, cancellation/reconnect, persisted task and undo are reported separately; original configuration/project are unchanged; missing evidence fails |
| P03 | Observable inference and explicit allowance recovery | TUI shows measured input or unknown, output reserve, remaining connection allowances and observed request stages; a user-confirmed finite addition preserves accounting, model and task; interrupted calls remain charged |
| P04 | Evidence-driven repeated-failure recovery | Actual diagnostics retain failed cases and source identity; unchanged failures and revisited source revisions produce actionable feedback without inventing fixes or preventing Full-access commands |
| P05 | Realistic compatibility and reliability qualification | Both provider adapters, longer conversations, interruptions and external edits are exercised; exact uncensored artifacts/settings are recorded for live repairs, including failures; native transport and repair quality stay separate |

All five orders include documentation and appropriate automated checks. Physical
Pascal GPU throughput/fit and human novice usability remain PC measurements.
Training remains a separate handoff until enough rights-reviewed, independently
verified families and a supported training environment are available.

Implementation for P01–P05 is complete. The
[retained evidence](evidence/pc-ready/README.md) records 123 passing Rust tests,
strict Clippy/formatting, the entire reusable setup, both adapters, twelve-turn
Git fixtures, narrow-terminal allowance recovery, thirty UI/PTY stress rounds,
sixty stream scenarios and real-model outcomes. The release workflow rebuilds
the exact clean beta tag, checks the package against the attested public prior
release and verifies workflow-identity provenance before publication. Its
attached receipts qualify the downloadable package separately from the local
development archive.

Language-tool preflight now distinguishes missing/broken Rust or Node from model
behavior before issuing a request. An initial Rust infrastructure failure and
the exact saved-source recheck are preserved separately from the requalification.

Beta 2 publication was cancelled after tag verification exposed a procfs race
in a cleanup probe. The corrected beta 3 adds five regression checks and repeats
clean-tag publication; the failed log and correction are retained in
[release finalization evidence](evidence/beta3-finalization/README.md).
