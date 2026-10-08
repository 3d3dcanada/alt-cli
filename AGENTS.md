# Working on Alt

Alt is a Rust terminal workspace for people using small local models, including
people unfamiliar with programming. Keep the first-run UI useful without a
configuration file. Prefer labelled choices and recoverable errors over requiring
shell commands or TOML edits. Never lose an unsent message after a connection error.

The reference target is Linux, GTX 1070 8 GB VRAM, and 16 GB RAM. Cloud CPU results
do not establish that machine's performance. Retain CPU operation. Every live
model evaluation must explicitly use an uncensored/abliterated checkpoint, as
requested by the owner. Deterministic protocol fixtures have no model weights.

Use the existing checkout; do not create a worktree unless requested. In the cloud:

```bash
export CARGO_HOME=/workspace/.cargo
export RUSTUP_HOME=/workspace/.rustup
export PATH="/workspace/.cargo/bin:/workspace/.alt-tools:$PATH"
export PYTHONPATH="/workspace/.alt-tools/python${PYTHONPATH:+:$PYTHONPATH}"
```

Run `bash scripts/setup-cloud.sh` for repeatable setup. After changes, run relevant
Rust tests, formatting, and strict Clippy. For UI changes run `scripts/smoke_tui.py`
and `scripts/smoke_product_tui.py` (all pages, settings, mouse and compact navigation).
For engine changes run `scripts/smoke_goose.py` for both `openai` and `ollama`.
For task/access/memory UI changes also run `scripts/smoke_task_tui.py`.
Project policies/journals live in project.rs, the MCP boundary in toolbox.rs, and
execution/isolation in sandbox.rs. Full access must stay capable of arbitrary
commands and network access; do not turn optional guided guards into global limits.
The opt-in live scripts and their limits are documented in `docs/LIVE_EVALUATION.md`.

Keep Goose-specific behavior in `engine.rs`, provider/model acquisition in
`models.rs`, owned inference processes/installers in `runtime.rs`, and background
agent work in `workspace.rs`. The UI should remain responsive during network,
download, model-loading, and inference operations. Preserve raw evidence separately
from bounded UI previews. Treat model success claims as unverified until tool and
independent check results support them. Record failed evaluations as well as passes.

Preserve artifact integrity checks and OS certificate trust. Never disable TLS
verification. Do not silently switch providers/models or auto-enable all tools.
Document isolation precisely: Guided checks use Bubblewrap and fail closed when unavailable; Full access terminal/checks use host permissions. Never silently downgrade isolation. Recheck license/attribution
requirements before distributing upstream binaries with Alt.
