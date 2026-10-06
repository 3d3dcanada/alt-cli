# Implementation and verification — Alt 0.5 beta

October 6, 2026. The five approved audit priorities are implemented and locally
validated. This is an unpublished beta, not a claim of general coding reliability,
physical GPU qualification or completed human usability acceptance. The
[work orders](WORK_ORDERS_0.5.md) separate implementation from those release gates.
The [0.4.1 implementation](IMPLEMENTATION_0.4.1.md) and
[audit](AUDIT_0.4.1.md) remain available as history.

## What changed

| Priority | Delivered behavior | Implementation and evidence |
|---|---|---|
| Explicit verification | Checks declare tests/build/lint/health/custom. Tests need fresh bounded JSON/JUnit/TAP cases. Independent assertion files are pinned outside source. Command success, required-check completion and behavioral acceptance have separate badges and export fields. Changed assertions, contracts, source or installed dependencies stale the result. | `src/verification.rs`, `project.rs`, `project_services.rs`; [contract guide](VERIFICATION_CONTRACTS.md), `tests/verification.rs`, [TUI walkthrough](evidence/v5/tui-contracts.log) |
| Responsive UI | Project workers wait cooperatively for locks, support cancellation and discard results from old project generations. File browsing, search, editor operations, context, checks, exports, history, conversation loading, brief writes and language rename work run on workers. Pending refreshes coalesce; background errors preserve forms. Atomic commits report their actual outcome. | `src/project_worker.rs`, `src/tui/`; [30 measured journeys](evidence/v5/ui-pressure.json), database-contention and stale-generation regressions |
| Small-model assistance | User-selected All/Inspect/Coding/Terminal tool focus, actual check feedback, repeated-failure guidance, exact configuration identity and an explicit optional reasoning switch. Twenty development fixtures plus four held-out fixtures have independent oracles; campaigns freeze inputs, support resume and retain failures. | `src/capability.rs`, `toolbox.rs`, `engine.rs`; [evaluation procedure](EVALUATION_MATRIX.md), [matched pilot comparison](evidence/v5/spark-default-comparison.json), [claim review](evidence/v5/claim-review.json) |
| Runtime qualification | CLI/TUI measure the selected model at requested contexts, native context where observable, load time, sampled process RAM/VRAM, token generation, cancellation after a generated token, process cleanup and restart. Errors remain saved; external runtime ownership stays explicitly unmeasured. | `src/qualification.rs`, `benchmark.rs`, `runtime.rs`; [qualification guide](QUALIFICATION.md), [raw measurements](evidence/v5/qualification/) |
| Release and recovery | Embedded compile-time source/toolchain provenance, stale-binary rejection, resolved CycloneDX SBOM, portable build and signed-candidate workflows. Schema guards precede writes; related records and preferences commit together. Relocation journals preserve originals. Failure injection covers settings, backup/restore and installation. Parser mutation tests, lifecycle soak and a novice-study recorder are included. | `build.rs`, `schema.rs`, `storage.rs`, `models.rs`, `scripts/`; [package checks](evidence/v5/package-prereport.json), [soak](evidence/v5/soak-resources.json), [novice protocol](NOVICE_ACCEPTANCE.md) |

Full access retains arbitrary host commands, networking, installations and external
tools. Focused tool menus and Guided/Review access are explicit choices. No model,
provider or reasoning preset is substituted automatically. The default reasoning
choice preserves the selected template's behavior.

## Local validation

- **79 Rust tests passed**, including structured-report rejection, stale assertions
  and dependencies, legitimate build-only checks, newer-schema rejection, failure
  recovery, worker contention, cancelled conversation loading, bounded protocol
  parsing, process cleanup and model acquisition. Formatting and strict Clippy pass.
  A later targeted runtime regression also covers the observed missing-library error.
- **Four real PTY walkthroughs passed**: first-run/connect/repair/resume; Task and
  access/verification; files/context/extensions/terminal; independent assertion and
  tool-focus/reasoning configuration. These use real terminal input and persisted
  evidence, not screenshot text alone.
- **30/30 pressure journeys passed** at 120×40, 80×24 and 60×18 with project-lock
  contention. Input-to-visible p95 was **43.85 ms**, maximum **48.11 ms**;
  cancellation-to-visible p95 was **65.04 ms**, maximum **69.14 ms**. These include
  Python observation and describe this cloud run, not every computer or operation.
- **30/30 PTY lifecycle rounds passed**, with process identity, descriptor counts,
  sampled RSS, retained state size and post-stop process checks. A Linux subreaper
  gives test-owned descendants the cleanup normally supplied by a proper init.
  Measurements are point samples; this is a bounded soak, not proof of no leaks
  during indefinite inference, downloads or use.
- All **24 independent fixture oracles** rejected their broken seeds and accepted
  known repairs. Actual Goose OpenAI-compatible and Ollama protocol fixtures passed;
  they have no model weights and establish protocol behavior only.
- The locked dependency audit reported **zero advisories and zero warnings** for
  358 dependencies. The dated advisory database identity is retained. The resolved
  runtime/build SBOM contains 314 components and passes the hash-pinned official
  CycloneDX 1.6 schema. Development dependencies remain in license notices.
- The packaged frontend ran on Debian 11 and Ubuntu 24.04. Both hosts also passed
  actual installation, an installed TUI and prior-version state recovery. Tests used
  the actual 0.4.1 archive for upgrade, rollback with its matching state backup,
  re-upgrade, every-file tamper rejection and an installed TUI walkthrough.
  Simulated ENOSPC and death around executable replacement left a complete old or
  new executable. A separately retained ephemeral-key package passed archive and
  installer signature/tamper checks; it is not a production signing identity.

Raw logs and selected model traces are in [evidence/v5](evidence/v5/README.md).
Their manifest identifies every retained file. Reports carry exact binary/source
identities where supported, including earlier candidate builds; the final archive
is checked separately in `dist/release-gate.json`. Repackaging identical inputs is
checked for identical archive bytes. Independent compiler-build reproducibility
and a remote GitHub Actions run are not established by local checks.

## What the uncensored-model trials actually showed

Every live run selected SHA256-verified uncensored/abliterated weights explicitly.
There was no standard-model or hosted fallback.

The matched Spark-X2.5 1.7B Q4 pilot used an 8K native context, two CPU threads,
Goose 1.53.0, llama.cpp b11429 and five attempts on the same Python feature task.
The 0.4.1 baseline passed **0/5**, and the 0.5 coding-focus candidate passed **0/5**.
Most attempts reached an output limit or timed out without implementing a repair.
Both are explicitly incomplete pilots of a planned 100-attempt development matrix.
The 95% Wilson interval for each observed 0/5 is 0–43.45%, for this task and sampled
conditions only; it is not a population-wide coding estimate.

A separate **reasoning-off Spark pilot also passed 0/5**. It produced more edits,
but failures included a missing import, invalid syntax and incorrect behavior.
Two of its five attempts claimed implementation completion despite failed source
assertions; one had never applied the proposed change. The retained claim review
checks those claims against actual files and oracles. Other attempts made no
completed-repair claim, or produced no prose. This is not an exhaustive assessment
of every sentence. Disabling reasoning is therefore an available experiment,
**not a recommended preset or demonstrated accuracy improvement**.

Coding focus deliberately exposes named checks and omits arbitrary terminal
commands. No named check was configured in these pilots. Spark repeatedly asked
for terminal execution it had not been offered. The suite retains this failure;
it does not automatically expand tools or silently change the comparison. Users
can choose All/Terminal or configure checks themselves.

The CPU qualification suite separately succeeded at 2K/4K/8K for Spark and
Qwen3-4B-Instruct-2507 Heretic Q4. Each row includes three short generation samples
and a generated-token cancellation/restart check. Spark used roughly 1.92–2.00 GiB
sampled runtime RSS and 19.0–23.3 generated tokens/wall-second; Qwen used
4.35–5.20 GiB and 9.1–10.8 tokens/wall-second. Some Spark samples contain only
reasoning before their output cap. Generation passed means the protocol generated
tokens, not that the requested answer or a coding task was completed.

A real 512 MiB cgroup limit produced the expected RAM preflight failure, retained
its report and left the selected model unchanged. Minimal Ubuntu also reproduced
a missing `libgomp.so.1` runtime dependency. Alt now names the relevant system
package in the error. That negative result is retained separately from successful
runs with required libraries present. Both models also passed at a measured native 16K context in Debian 12 containers:
Spark under an 8 GiB/2-CPU budget used 2.11 GiB sampled runtime RSS; Qwen under a
16 GiB/2-CPU budget used 6.33 GiB. Each passed generated-token cancellation and
restart. These are real container limits, not physical 8/16 GiB PC qualifications.

The cloud has no physical GPU. CPU quota, shared build activity and container
limits are recorded and must not be extrapolated into GTX 1070 speed or memory fit.
The earlier MiMo exploratory evidence remains in [LIVE_EVALUATION.md](LIVE_EVALUATION.md);
no new MiMo artifact or broad reliability result is implied here.

## Open acceptance gates and operational boundaries

1. Complete the 20-task × five-attempt matrix and separate four-task held-out
   matrix before recommending an exact model/tool/template configuration. The
   three failed Spark pilots provide no basis for preset promotion.
2. Run the physical GTX 1070/8 GiB VRAM/16 GiB RAM target, older CPU-only computers
   and a newer GPU. Real LM Studio and credentialed private/gated Hub acquisition
   also remain unmeasured here. Reproduction procedures are supplied.
3. Observe at least five actual uncoached novice participants. The retained worksheet
   is explicitly unperformed. Terminal automation is not human usability evidence.
4. Run remote CI, configure the production signing identity and review the exact
   release candidate before publication. No source push, public release or
   environment publication occurred. Fresh-task snapshot restoration is unverified.
5. Extend bounded mutation/lifecycle testing into coverage-guided fuzzing and
   sustained streaming/download workloads on target hardware. Current corpus and
   sample limits do not establish indefinite resource stability.

The portable frontend requires glibc 2.30; the pinned managed Goose and llama.cpp
artifacts require 2.28 and 2.34 respectively. A frontend launch on an old distro
does not validate a newer runtime's system libraries. GPU builds remain a separate
explicit choice.

Verification establishes only the selected assertions. Dependency freshness is a
bounded local metadata inventory, not an attestation of every library or remote
service. Full access executes with normal account permissions and is not isolation
from hostile code. Guided checks fail closed when Bubblewrap is unavailable.
Atomic updates and retained originals aid process-crash recovery; these tests do
not establish recovery from every power-loss/filesystem/controller failure.
Rollback restores an executable, not a database downgrade: preserve newer state
and restore the corresponding prior backup into a new directory.
