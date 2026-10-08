# Frozen candidate delivery validation

These receipts measure the local candidate built from clean source commit
`16a51866306efd7fb22270181eafca2cf6bcd5f6`, source fingerprint
`662fd9f11392d39d05d6cea1185c8652830af4579cc3e34b248a3fccf782ecb1`.
The exact portable binary is
`54b0cf02702df83afa96c3c31b5905e2c5311d787b9400f7ac9ac745e0486e51`.
This candidate has not been published or attested as a new GitHub release.

- `release-gate.json`: all eight gates pass against the exact application archive
  and actual published beta 3. Includes full research/index binding, official
  CycloneDX schema, Debian Bullseye and Ubuntu 24.04 execution, installed TUI,
  prior-schema migration, automatic backup, rollback and re-upgrade.
- `aged-install-receipt.json`: published beta 3 to candidate generation, automatic
  backup restore, complete executable/documentation rollback and re-upgrade over
  the persistent longevity fixture. Every logical SQLite database and ordinary
  state file is unchanged. The restored core databases match the source; bulky
  inference exports remain explicitly separate from core backup.
- `package-footprint.json`: exact archive sizes, hashes and file counts. The small
  application archive retains all user guides; the separate research archive
  accounts for every one of 9,436 source documentation/evidence files present at
  packaging time, including baseline failures.
- `draft-tui-final-portable.json` and `recovery-tui-final-portable.json`: both newly
  wired CI terminal smoke suites also pass against these exact portable binary
  bytes. They exercise durable/recovered drafts, manual model entry, raw output,
  check-input declarations, stored budgets, recovered files and explicit request
  corrections using keyboard PTYs and deterministic fixtures. Exact commands,
  script hashes and binary identity are in `portable-tui-invocations.json`.
- `reproduce-aged-install.py`: standalone source for the aged installation probe.
  Its `--cleanup-work` option removes only newly generated disposable package,
  installation and restored-state folders after success. The original fixture,
  original releases and JSON receipt are retained.

The aged fixture originally created 2,000 sessions and 8,000 deterministic events;
retention removed 1,000 archived sessions and one was restored. The installer gate
therefore preserves 1,001 live sessions and 4,004 events plus retained archives. It
does not open the newer-schema aged fixture using the old application; the
independent release-host gate measures actual prior-schema state migration.

The first release-gate attempt is retained. Seven gates passed; the package smoke
failed because its fault-injection harness imported the packaged Python helper
and added an unmanifested `__pycache__` file. The installer correctly refused that
modified package. The small import reproduction records both outcomes. Suppressing
bytecode before that test-only import fixes the harness; `tested-scripts.json`
records its exact source. The final successful run uses identical package and
binary bytes. No integrity checks were disabled.

These are deterministic software, installation and persistence measurements.
They do not measure model task quality, physical GTX 1070 behavior, machine power
loss, or success by uncoached novice users. Receipts added here after packaging are
not claimed to be inside the already measured research archive.
