# Beta 2 cloud completion evidence

This is fresh development qualification for the five
[PC completion work orders](../../PC_READY_WORK_ORDERS.md). It preserves failed
attempts and separates actual model quality from weight-free protocol behavior.
The source-input identity is recorded in `live/BUILD.json` and the finite trial
configuration in `live/PLAN.json`. The release workflow separately rebuilds the
exact clean tag and verifies its own package before publication.

## Application checks

- `rust-tests.log` / `final-rust-v2.log`: 123 Rust tests, all passed; format
  and final strict Clippy passed. The final allowance UI journey also passed
  plain-language inspection at both terminal sizes without sending a request.
- `training-tests.log`: 46 training-tool tests passed; no weights were trained.
- `dependency-audit.json`: no reported advisories or warnings, with database
  identity retained.
- `protocols/summary.json`: actual Alt/Goose OpenAI and Ollama protocols,
  native edits/checks, UI journeys and sixty stream-recovery attempts passed.
- `repository-openai.json` / `repository-ollama.json`: twelve-turn continuations
  with the full original goal, dirty Git files, an external edit and stale-handle
  recovery passed on both adapters.
- Allowance UI checks cover 120×40 and 80×24: observed waiting/receiving,
  exhaustion without another upstream request, confirmed finite addition,
  continued task, persisted allocation and explicit reconnect.
- PC recorder fixtures cover both adapters and deliberately wrong source with a
  normal native response. The latter must fail model correctness; it is a
  passing test of failure reporting, not a successful model repair.
- `setup-result.json` / `setup.log`: the entire documented setup command passed.
- `ui-pressure.json`: thirty terminal journeys under database contention passed.
- `soak.json`: thirty PTY lifecycle rounds passed with bounded process, file
  descriptor, memory and state-size observations.

`development-release-gate.json` describes the exact **local unpublished**
archive identified by its hash. Its seven checks include integrity, installation,
official SBOM schema, actual previous-artifact upgrade/rollback and old/current
Linux hosts. The published beta has separately attached package, provenance and
gate receipts; do not substitute the local archive hash for those release bytes.

`final-release-gate.json` records all seven passing gates for the final local
archive, now using the cryptographically verified **published beta 1** as the
actual upgrade/rollback prior. `setup-final-result.json` records the complete
final setup, including the added preflight tests and plain-language allowance UI.
The final local gate report is retained outside its tested archive; publication
rebuilds and gates the clean tagged archive with its own attached report.

This cloud blocks `tuf-repo-cdn.sigstore.dev`. `tuf-mirror/receipt.json` records
the official `sigstore/root-signing` GitHub commit used as a metadata mirror,
anchored in the exact Sigstore dependency of CLI 2.102.0. CLI verified the TUF
signature chain, expiry and target hashes before emitting a public trusted root.
The resulting root was used for normal bundle verification, preserving the
workflow/source-ref policy and TLS. The earlier failed mirror path attempt is
retained. Ordinary connected PCs and GitHub runners use the standard `gh
attestation verify` command.

Raw protocol stdout/stderr, terminal text and request/response receipts are
retained. Mutable database files, executable/weight files and Python bytecode are
excluded; exported task/session events and actual source/assertion files remain.
Paths under `/workspace` or `/tmp` identify the measured cloud run, not a startup
dependency for your computer.

## Fresh model trials

MiMo 9B and Josiefied 7B each passed the complete practice recorder. Repair,
follow-up and cancellation-resume each passed all four independent cases. Native
completion, persisted original goal, cancellation file preservation, undo,
unchanged assertions and normal settings all passed. Intermediate incorrect
7B edits and failed checks remain in the raw record. The first Rust trial lacked
the documented `CARGO_HOME`/`RUSTUP_HOME` environment: its original oracle could
not start Rust, and the model turn ended at the CPU deadline after twelve calls
(611.19 seconds including cleanup). That original report does not measure
behavior. `environment-recheck.json` separately tests the exact retained source
and unchanged oracle with the configured toolchain; an extreme-value case fails
(`Some(0.0)` instead of `Some(-0.5)`). The infrastructure requalification has a
separate plan/run with unchanged model, prompt, oracle, allowances and deadline.
All attempts are retained, with transport and behavior separate. Each trial keeps its
independent assertions, selected artifact, unchanged source goal, actual native
events and request/cost receipts. CPU trials use 8K context, 1,024 output tokens,
12 requests and 8,192 generated tokens **per connection**, and 600-second turn
deadlines. The practice recorder explicitly opens a new connection when resuming.
No model/provider fallback, changed oracle or automatic allowance addition occurs.

The configured Rust requalification passed language-tool startup and reproduced
the actual seed failure. Its resulting source still failed independent
acceptance, and normal native completion failed at the 600-second deadline after
twelve calls. This is a measured model/task failure. It remains separate from
the first infrastructure failure and the later check of that first saved source.

These are small development projects, not a matched improvement experiment or a
broader repository reliability claim. Earlier Spark/Qwen/MiMo/Josiefied failures
remain in their original cohorts. Physical GTX 1070 fit/speed, desktop RAM and
human novice usability remain local acceptance work. Training corpus review and
supported GPU training remain the existing handoff.

`live/FINAL-SOURCE-COMPARE.json` identifies the final UI-only allowance inspection
wording change after the frozen CLI build. Engine, inference, tools, prompts and
skills are unchanged. Final Rust/Clippy and actual allowance terminal checks repeat
on that source; the release workflow checks the exact clean tagged package.

`fixture-development/` retains earlier weight-free recorder prototype runs,
including incorrect fixture routing and a JSONL export parsing error. Those
failures led to the fixture/parser corrections; they are not real-model trials.
Final fixture outcomes live in `pc-recorder.json` and `protocols/`. No old failed
report was changed into a passing report.

`final-clippy.log` also retains an intermediate nested-format lint failure from
the UI wording edit. `final-clippy-v2.log` records its correction and strict pass.

`preflight-tests.log` covers unavailable tools, an executable Rust proxy with a
broken environment, and a live runner that stops before invoking its engine.
The runner now saves `environment-preflight.json` before any model request.
