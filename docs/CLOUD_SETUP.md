# Reusing the Alt cloud environment

The checkout is `/workspace/alt-cli`. Run the idempotent setup from that folder:

```bash
bash scripts/setup-cloud.sh
```

It pins Rust 1.99.0 and Goose 1.53.0, verifies the engine artifact, installs the
Python PTY-test and release-schema dependencies into `/workspace/.alt-tools/python`, and runs
Rust/build/lint, terminal walkthroughs, both historical and new independent
project oracles, training/schema/adapter boundaries, executable skill helpers,
bounded analysis and real-Goose provider/candidate fixtures, including both compact
edit formats through both adapters. It downloads no model weights. Package and
model-quality tests are separate because they take longer and need specific tools.

To retain raw compact protocol and TUI receipts, choose a fresh output directory:

```bash
ALT_SETUP_EVIDENCE_DIR=/workspace/alt-setup-evidence bash scripts/setup-cloud.sh
```

Use a different directory for another recorded run so earlier terminal evidence
remains intact. These scripted provider checks have no model weights and do not
count as coding successes.

For subsequent shells:

```bash
export CARGO_HOME=/workspace/.cargo RUSTUP_HOME=/workspace/.rustup CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export PATH="/workspace/.cargo/bin:/workspace/.alt-tools:$PATH"
export PYTHONPATH="/workspace/.alt-tools/python${PYTHONPATH:+:$PYTHONPATH}"
```

Use `cargo run -- --data-dir /workspace/.alt-data` for the UI, or pass the same
`--data-dir` to the packaged binary. This sandbox keeps the home directory read-only;
normal desktop installations can use the default per-user folder. Read `AGENTS.md` before
changes. Preserve project files and state; use temporary projects/data directories
for tests. Full access is available when explicitly selected. Guided isolation
requires working user namespaces/Bubblewrap and fails clearly when unavailable.
This cloud blocks that isolation directly; successful isolation evidence comes
from an explicitly namespace-enabled disposable container.

Optional validation:

```bash
# Docker is needed; the portable script builds and packages against Debian 11.
bash scripts/build-portable.sh
python3 scripts/release_gate.py dist/alt-0.6.0-linux-x86_64.tar.gz \
  --previous /path/to/actual-previous-package.tar.gz --require-upgrade \
  --output dist/release-gate.json
# For real SSH/tmux acceptance, build the disposable loopback-only test host.
python3 scripts/build-terminal-host.py
python3 scripts/smoke_terminal_hosts.py --alt target/portable-glibc231/release/alt
# Requires an installed rust-analyzer and Rust toolchain.
python3 scripts/smoke_language.py --alt target/debug/alt
```

Browser/security acceptance additionally needs Node/Playwright/Chromium, Semgrep,
pip-audit and npm advisory access. `scripts/smoke_packs.py` and
`scripts/smoke_auditors.py` document their command arguments. These optional tools
are not setup prerequisites or bundled dependencies. The terminal-host helper
passes the existing proxy configuration without printing credentials and copies
public OS trust roots into its temporary build context; it never disables TLS.

For optional advisory tools in this read-only-home cloud, keep their caches in
the workspace: set `XDG_CACHE_HOME=/workspace/.alt-tools/cache`,
`PIP_CACHE_DIR=/workspace/.alt-tools/cache/pip` and
`npm_config_cache=/workspace/.alt-tools/cache/npm`. This does not change `$HOME`
or certificate verification. `scripts/smoke_auditors.py --skill dependency-review`
exercises the declared skill path with actual registry/advisory responses.
Semgrep also needs `SEMGREP_LOG_FILE=/workspace/.alt-tools/cache/semgrep.log`
and `SEMGREP_SETTINGS_FILE=/workspace/.alt-tools/cache/semgrep-settings.yml`.
For the optional Docker release gate, use
`DOCKER_CONFIG=/workspace/.alt-tools/docker` to keep Buildx state writable.

All real model tests require an explicitly selected uncensored/abliterated artifact
and its exact SHA256. See `LIVE_EVALUATION.md` for commands and actual failures.
No standard/cloud fallback is authorized. Saved GGUF/runtime files live outside
the repository; they are not included in the Alt package. A CPU cloud test cannot
validate the reference GTX 1070/8 GiB VRAM/16 GiB RAM machine.

Cloud configuration changes are saved as a reviewable draft. Saving the draft
does not publish or apply it. Review and save it in environment settings, then
publish to activate the setup and filesystem snapshot. Repository commits and
local package checks do not publish that environment; fresh-task snapshot restoration remains
unverified until the user publishes and starts a new task.
