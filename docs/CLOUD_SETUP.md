# Reusing the Alt cloud environment

The checkout is `/workspace/alt-cli`. Run the idempotent setup from that folder:

```bash
bash scripts/setup-cloud.sh
```

It pins Rust 1.99.0 and Goose 1.53.0, verifies the engine artifact, installs the
Python PTY-test and release-schema dependencies into `/workspace/.alt-tools/python`, and runs
Rust/build/lint, four terminal walkthroughs, 24 independent project oracles and
both real-Goose protocol fixtures. It downloads no model weights. Package and
model-quality tests are separate because they take longer and need specific tools.

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
python3 scripts/release_gate.py dist/alt-0.5.0-linux-x86_64.tar.gz \
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

All real model tests require an explicitly selected uncensored/abliterated artifact
and its exact SHA256. See `LIVE_EVALUATION.md` for commands and actual failures.
No standard/cloud fallback is authorized. Saved GGUF/runtime files live outside
the repository; they are not included in the Alt package. A CPU cloud test cannot
validate the reference GTX 1070/8 GiB VRAM/16 GiB RAM machine.

Cloud configuration changes are saved as a reviewable draft. Saving the draft
does not publish or apply it. Review and save it in environment settings, then
publish to activate the setup and filesystem snapshot. This work did not push a
commit or publish an environment/release; fresh-task snapshot restoration remains
unverified until the user publishes and starts a new task.
