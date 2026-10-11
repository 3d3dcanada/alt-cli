# Alt CLI 0.7 handoff (2026-10-11)

For the cloud agent picking up Alt CLI. Read this, then `AGENTS.md`/`CLAUDE.md` if present, then the files named below.

## Where things stand
- `main` (pushed 2026-10-11, `04712f7`) = 0.6.0 + the Neon interface set N01-N09 (`docs/NEON_WORK_ORDERS.md`):
  model picker metadata, never-dead-end launch, theme registry, settings tabs, Ctrl+P palette, skills/tools
  manager, layout modes. This is the base. Build on it.
- Three branches were never merged and overlap with main. Harvest, don't merge blind:
  - `build/final-pass-2026-10-08` (14 commits): saved themes, layouts and a graphical Home dashboard. It
    predates the Neon set and covers some of the same ground. Take only what main lacks (the Home dashboard is
    the likely keeper); main's theme engine wins on conflicts.
  - `audit/final-pass-2026-10-08`: the beta 3 final audit and prioritized improvement work orders. Read it;
    fold its still-open items into the board below.
  - `review/2026-10-08`: `docs/WORK_ORDERS_0.7.md` (A01-A10) and the PC model trial results
    (`docs/research/2026-10-08-pc-model-trial/README.md`). None of A01-A09 is built yet.

## The owner's direction
Alt CLI stays public: a powerful coding agent for local and uncensored models. "As powerful as possible", with a
much better interface. It must work with any model on any server (broad parameter support), NVIDIA and other
hosted APIs as well as local llama.cpp/Ollama/LM Studio. The PC trial showed small local models fail on
reasoning cut-offs and edits (Spark 4B 0/8 at 1,024 output tokens, MiMo 9B abliterated 3/8, hosted Kimi K3 4/8,
GLM 5.3 3/8). A01-A06 attack exactly that.

## Do, in order (one commit per order, tests with each)
1. Bring `docs/WORK_ORDERS_0.7.md` and the trial README onto main. Add the audit branch's open items and the
   Home dashboard harvest as numbered orders, and mark which overlap with N01-N09 and are already done.
2. A07 security fixes first, then A01-A06, then A09 (README + repository weight), then the harvested items.
3. A08 (managed CUDA 12 runtime) and A10 (matched re-run) need the owner's PC with a GPU: prepare them
   (code, script, labels) and leave the run to the owner.
4. Interface: every new capability is reachable from the TUI (palette, settings tabs, mouse and keyboard), uses
   the theme engine, and stays usable at 80x24.

## Rules
- No new dependencies without a written reason in the commit. No model weights or large binaries in git;
  evidence over 1 MB goes to a release asset or stays out.
- Fixtures only in the cloud: no API keys, no GPU. Never commit secrets; run a secret scan before every push.
- Gates per order: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, the TUI smoke
  scripts, and screenshots at 80x24 plus one wide size.
- Work on branch `cloud/0.7-2026-10-11`; open one PR to main with a table: order / commit / test / result /
  deferred. Don't merge it; the owner reviews. Don't publish a release or tag.
