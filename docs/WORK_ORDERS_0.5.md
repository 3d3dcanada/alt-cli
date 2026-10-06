# Alt 0.5 — approved reliability work

The owner authorized planning and implementing all five priorities from the 0.4.1
audit. These orders include audit R01–R08. Implementation, local validation, live
model measurements, physical hardware qualification, and publication are separate
statuses. Full access continues to provide arbitrary host commands and networking.

## 01 — Explicit verification contracts

- [x] Classify checks as tests/build/lint/health/custom without breaking saved commands.
- [x] Parse bounded structured JSON/JUnit/TAP reports; reject missing, malformed,
  incomplete, contradictory, failing and zero-execution test reports.
- [x] Support an independently pinned assertion outside model-editable project source.
- [x] Fingerprint contracts, assertion files and installed dependency state; show stale
  evidence after changes or verifier upgrades.
- [x] Separate command success, required-check completion and behavioral acceptance
  in CLI, Task UI, model feedback and exports; provide a labelled configuration wizard.
- [x] Cover fabricated summaries, changed assertions, dependency edits and valid
  build-only tasks with regression tests.

## 02 — Responsive project operations

- [x] Add a cancellable project worker with explicit lifecycle and bounded lock waits.
- [x] Move project file/history/context/verification and conversation operations onto workers.
- [x] Tag updates with operation and project generation; discard stale view updates.
- [x] Coalesce refreshes, preserve drafts during background errors and report contention
  inline. Atomic commits finish before cancellation is reported.
- [x] Run 30 terminal journeys under pressure and retain every attempt; measure input
  latency and cancellation acknowledgement separately from task completion.

## 03 — Measured small-model assistance

- [x] Add exact-configuration capability records with artifact/runtime/template provenance.
- [x] Add explicit focused tool profiles, current-evidence feedback and repeated-failure
  guidance without switching the selected model or removing Full access.
- [x] Expand independent project fixtures to at least 20 development tasks plus held-out
  tasks, including setup, dependencies, migrations and services; self-test every oracle.
- [x] Add resumable matrix execution, five attempts per configuration, immutable attempt
  evidence and separate source/turn/claim scores; comparisons require matching settings.
- [x] Run accessible uncensored-model evaluations and report failures and uncertainty.
  Do not promote a preset on a native tool probe or a few successful cases.

## 04 — Hardware and provider qualification

- [x] Build a qualification runner for selected CPU/GPU runtime settings and native
  contexts, with provenance, memory/throughput, stop/restart and failure evidence.
- [x] Generate measured compatibility rows and user-facing fit guidance from real reports.
- [x] Exercise available CPU and constrained-memory paths plus deterministic runtime
  failure cases; distinguish resource limits from physical-device evidence.
- [x] Supply reproducible GTX 1070/newer-GPU, LM Studio and private-Hub qualification
  commands; actual unavailable hardware/credentials remain pending, never simulated passes.

## 05 — Release, recovery, fuzzing and novice acceptance

- [x] Embed source/commit/dirty/toolchain/target provenance at compile time; reject stale
  same-version binaries during packaging and generate a resolved dependency SBOM.
- [x] Build the portable artifact in CI; validate exact packages on old/current Linux,
  real prior-version upgrades and rollback, and wire production signing to an explicitly
  invoked release workflow with configured credentials.
- [x] Reject newer state before schema mutations, make related database writes transactional,
  and exercise fault recovery for settings/backups/restore/relocation/install.
- [x] Add bounded parser/property fuzz coverage and scheduled lifecycle/streaming soak tests.
- [x] Add a novice acceptance protocol, guided diagnostics and reproducible keyboard/narrow/
  Unicode/error-recovery journeys. Actual uncoached human sessions remain a separate gate.
- [x] Run checks, retain negative evidence, package the result and record remaining gaps.

## Execution record

Implementation and local checks are complete; details and evidence are in the
[0.5 implementation report](IMPLEMENTATION.md). Checked items mean the named
implementation and bounded local checks were delivered, not that every release
acceptance gate below passed.

- [ ] Complete development and held-out model matrices before promoting a preset.
- [ ] Measure physical GTX 1070, newer GPU and older CPU machines.
- [ ] Exercise a real LM Studio instance and authorized private/gated Hub account.
- [ ] Record actual uncoached novice sessions.
- [ ] Run remote CI, establish production signing, review and publish a release.
- [ ] Extend bounded mutation/lifecycle checks to sustained workload and coverage-guided fuzz campaigns.

All three Spark pilots failed 0/5 on the sampled feature task. No improved preset,
remote CI result, physical GPU result, human session or publication is implied.
