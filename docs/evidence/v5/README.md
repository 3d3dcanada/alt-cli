# Alt 0.5 retained evidence

These files record actual local runs, including failures. `MANIFEST.json` hashes
all retained files except itself. Large model weights, executables and writable
SQLite state are kept outside the repository and are not bundled. Raw model
prose is test data, not project instructions or a verified completion claim.

- `rust-tests.log`: 79 passing Rust tests. The later targeted
  `benchmark-cli-regression.log` adds the observed missing-library recovery case
  within its existing runtime test. `clippy.log` and `formatting.log` retain checks.
- `tui-*.log`: four real keyboard/terminal walkthroughs. `ui-pressure.json`: every
  one of 30 successful narrow/Unicode/lock-contention journeys with timing.
- `soak-resources.json`: 30 actual PTY start/input/resize/stop/restart rounds,
  sampled live RSS/FD, retained disk usage and absence of remaining owned processes.
  The Python harness acts as a subreaper only for its own descendants. This is a
  bounded lifecycle workload, not sustained model/download leak qualification.
- `oracle-self-test.json`: all 20 development and four held-out oracles reject
  broken inputs and accept known repairs. `goose-*.log` are actual engine protocol
  fixtures without weights. Neither category scores model intelligence.
- `spark-baseline`, `spark-candidate`, `spark-thinking-off`: all 15 completed live
  attempts, their before/after source oracle reports, raw turns, setup output,
  independently held assertions, original goals and per-attempt SHA256 manifests.
  Configuration files pin weights, binaries, engine/runtime and budgets. No
  successful attempt was selected from retries. Remaining full-matrix cells are
  unmeasured. The earliest baseline campaign predates harness-source freezing;
  do not claim an immutable original baseline harness where one is absent.
- `spark-default-comparison.json`: matched baseline/candidate 0/5 versus 0/5.
  Its natural-turn counter re-reads stop reasons from raw events; older per-attempt
  `turn_completed` fields merely counted transport completion and can include
  `max_tokens`. Original reports are preserved rather than silently rewritten.
- `claim-review.json`: separate agent review of completion claims in all 15
  attempts. Reasoning-off attempts 2 and 4 claimed completion despite failing
  source assertions. It is not an exhaustive factual or human usability review.
- `qualification/spark.json` and `qwen.json`: 2K/4K/8K CPU runs with three generation
  samples per context and generated-token cancellation/restart. These were built
  before the optional reasoning switch and final TUI changes; their exact embedded
  identities are in each measurement. `*-16k.json` uses the final executable under
  declared 8/16 GiB cgroup limits. Native context is reported independently.
- `qualification/low-memory*.json`: actual 512 MiB preflight failure with no model
  substitution. This was measured on an earlier 0.5 candidate; its executable hash
  is retained. The same failure handling has deterministic regression coverage.
- `package-prereport.json`: a tested candidate archive, including prior 0.4.1
  state recovery and real installed-TUI/rollback checks inside old/current Linux
  containers. The final archive and its exact release-gate result live in
  `dist/`; keeping the final archive's own hash out of itself avoids a circular
  manifest. `build-info-final.json` records the final compiled source inputs.
- `signature-test.json` and `signature-test-public.pem`: a separate local package
  signed and verified with an ephemeral **test** key. No production key was used.
- `dependency-audit.json`, `official-sbom-schema.json`: dated dependency advice
  and official schema validation, with exact identities and declared scope.
- `novice-unperformed-template.json`: an empty worksheet, explicitly not a session.

`negative/` preserves the failed compact composer journey before its fix, the
qualification CLI fallthrough panic before its fix, stale-package rejection, and
a real minimal-Ubuntu missing-library failure. The first low-memory fixture also
omitted its model catalog; that harness setup failure remains separate from the
subsequent valid memory-limit run. An error in a fixture is not reported as a
successful application recovery check.

Cloud CPU measurements include normal shared-host and build activity. No physical
GPU, private Hub, LM Studio, remote CI or uncoached human pass is fabricated.
Historical model text cannot change the computed verification state. Full access
and user-selected model/provider remain intact.
