# Contributing to Alt

Read [AGENTS.md](AGENTS.md) and the [architecture map](docs/ARCHITECTURE.md) before
changing behavior. Keep first-run setup useful without a model or a configuration
file. Preserve unsent text, imported originals, raw evidence and recoverable errors.

Use an issue to describe a reproducible problem or a specific improvement, then
open a focused pull request. Include what changed, why and the checks actually
run. Do not claim physical-device, provider or model results that were not measured.

## Local development

Install Git, a C compiler, Rust/rustup and Python 3.11+ for the complete development
harness. Python 3.11 is a test/packaging requirement; the package installer itself
also runs on the Python 3.9 host used in Debian 11 validation.

```bash
git clone https://github.com/3d3dcanada/alt-cli.git
cd alt-cli
cargo build --locked
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
```

Create a Python environment for terminal/schema checks:

```bash
python3 -m venv .venv
source .venv/bin/activate
python3 -m pip install -r requirements-dev.txt
python3 scripts/smoke_tui.py
python3 scripts/smoke_product_tui.py
python3 scripts/smoke_task_tui.py
python3 scripts/smoke_workbench_tui.py
python3 scripts/smoke_verification_tui.py
python3 scripts/acceptance_projects.py
```

Use disposable project/state directories for tests. `ALT_TEST_BINARY` selects a
built executable for supported terminal scripts. Protocol fixtures use no model
weights. Engine changes also need the actual Goose protocol fixtures:

```bash
python3 scripts/smoke_goose.py --goose /path/to/goose
python3 scripts/smoke_goose.py --goose /path/to/goose --provider ollama
```

Use the version and integrity pins in [THIRD_PARTY.md](THIRD_PARTY.md). The prepared
cloud has a separate, repeatable [setup procedure](docs/CLOUD_SETUP.md). Ordinary
desktop contributors should not run its `/workspace` installation script.

## Which checks matter

- Run the relevant Rust regressions, formatting and strict Clippy after changes.
- UI changes need a real PTY walkthrough. Task/access/memory changes also need the
  Task walkthrough; check-contract changes need the verification walkthrough.
- Preserve negative cases for recovery, bounded parsing, stale evidence and
  interrupted operations. Prefer assertions about observable behavior.
- Model quality requires separate [independent project oracles](docs/EVALUATION_MATRIX.md).
  Every live evaluation in this project must explicitly select a verified
  uncensored/abliterated checkpoint; never substitute a standard/cloud model.
- Keep historical raw evidence immutable. Save new reports or a separate reassessment
  with hashes and scope. Synthetic protocol passes are not inference-quality passes.
- Hardware and usability claims need actual [qualification](docs/QUALIFICATION.md)
  and [novice sessions](docs/NOVICE_ACCEPTANCE.md); automation is not a substitute.

## Implementation conventions

Keep Goose behavior in `engine.rs`, acquisition in `models.rs`, owned inference
in `runtime.rs`, project state/evidence in project services and UI rendering in
`src/tui/`. Use background workers for slow file, database and network work.
Tag asynchronous results so old projects cannot replace the current view. Preserve
forms after errors and report atomic mutations honestly when cancellation races them.

Full access must retain arbitrary host commands/network use. Guided isolation and
focused tool inventories are user choices. Preserve TLS, artifact checksums,
process-ownership checks and assertion freshness; do not silently downgrade
isolation, substitute a model or interpret model prose as successful verification.

Do not commit model weights, generated state, API keys, build caches or release
archives. Keep `Cargo.lock` in version control for this application. Review the
staged files and third-party licenses before a pull request.

## Packaging and release work

A host package can be built with `python3 scripts/package-linux.py`; its ABI
requirement reflects that host. The prepared `/workspace` environment can run
`bash scripts/build-portable.sh` for the pinned Debian 11 build. The GitHub Verify
workflow runs that portable path and uploads an unsigned development artifact.

`release_gate.py` exercises an exact package and accepts `--previous` for real
prior-version state recovery. The signed-candidate workflow requires an actual
previous-package URL/checksum and a configured release signing identity. This
first source publication does not invent a previous published binary release.
Workflows produce reviewable candidates; they do not publish a release automatically.

A release needs its exact tested archive, build identity, dependency/SBOM evidence,
matching upgrade/rollback fixtures and explicitly recorded outstanding gates.
Repository source publication and binary-release qualification are separate steps.
