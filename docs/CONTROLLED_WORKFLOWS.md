# Alt 0.3: project work and Full access

This is the historical foundation. Use the [0.4 workspace guide](WORKSPACE_GUIDE.md)
and [current implementation record](IMPLEMENTATION.md) for expanded behavior.

October 6, 2026. This replaces the 0.2 tool boundary. The app is still Rust/Ratatui,
with Goose 1.53.0 over ACP, but **Alt now owns its tools and execution decisions**.
The original research and unsuccessful model evaluations remain in this repository.

## Choose how to work

Open **Settings → Access**:

| Mode | What it can do |
|---|---|
| Review only | List, read, search, explain, and remember plans/decisions |
| Guided changes | Review file edits with checkpoints; approve configured checks in an isolated source copy |
| Full access | All the above, plus arbitrary Bash commands, networking, dependency installation, and external tools with your account's permissions |

Full access is the broad-capability mode requested for this product. It does not
filter task topics or model families. An approval can authorize one action, and
**Trust session** authorizes subsequent requested actions in the selected mode.
Changing access reconnects the engine and resets session trust. CLI equivalent:

```sh
alt --access trusted run 'Work on this project' --allow-tools
```

`trusted` is the CLI spelling for Full access. A bare `alt run` denies requested
mutations/execution. Inspection tools do not need approval. There is no hidden
cloud/model fallback. Any compatible model ID is accepted; endpoint/template/tool
support must actually work. Uncensored claims only gate the explicitly requested
live evaluation, not ordinary connections or conversations.

Terminal commands run in the **real project**, inherit your environment, and have
normal host/network access. They are not checkpointed and can affect other files,
services, databases, or remote systems. Stop kills the owned process group; it
cannot undo completed effects or guarantee control of a deliberately detached
process. Use the reviewed edit tool for file-version undo.

## Task progress

Open **Task progress** (Alt+8), or the Workspace's Task button. It shows the goal,
Inspect → Plan → Edit → Test → Results phase, current evidence, next action, and
tracked changes. The application derives edit/check status from its own records;
a model's claim of success cannot turn a failed check into a pass.

- **Configure checks** offers project-specific suggestions and a custom command.
  Registration does not execute anything. Commands can also be configured with
  `alt task configure-check 'Tests' -- python3 check.py`.
- **Run checks again** runs the selected registered command directly, without a
  model call. **Evidence** opens exit status, output, timeout/cancellation, source
  snapshot, execution mode, and elapsed time.
- **Diff** shows a saved action. Test/assertion edits are visibly marked for
  separate review; changing tests is not silently equated with fixing a bug.
- **Undo edit / Undo task** restores original contents only when current files
  still match Alt's postimages. Known conflicts stop task undo before it starts.
  A crash during a sequence can still leave a partial rollback; the journal
  recovers each action and shows its state. Unrelated uncommitted work is preserved.
- **Remember** saves a user decision across conversations in the same project.
  **Memory** searches current text/function names and the task ledger locally.
- **Export** saves goals, plans, next steps, decision provenance, original/changed
  file versions, and captured check evidence as JSON.

Task-page shortcuts: R run checks, C configure, Enter diff, U (uppercase) undo task,
u undo edit, E evidence, M memory, N remember, X export. Mouse buttons are labelled.

## Tool boundary and recovery

Goose's developer extension is disabled in an Alt-owned `engine-v3` configuration.
Only these seven MCP tools are loaded: `list`, `read`, `search`, `remember`, `edit`,
`run_check`, `terminal`. The stdio MCP process forwards calls over a private Unix
socket to its owning Alt process. There is no public tool listener. Approval
handling belongs to Alt, so MCP read-only annotations/Goose classification cannot
bypass mutation/execution approvals. The annotations avoid duplicate approval UX;
they are not used as authorization.

`read` records the current file hash. `edit` requires a recorded plan, validates
arguments, checks the recorded hash against current contents, and replaces one
unique exact substring (or explicitly creates/deletes a file). A caller may supply
an additional expected hash, but small models do not need to reproduce it. Each
proposal stores preimage/postimage and a diff **before** approval. Approval is
one-use; Alt checks the file again immediately before applying. Repeating an
already applied proposal cannot apply it twice. Terminal access remains available
in Full access for broader operations, including operations outside these contracts.

Confined file operations use `cap-std`; absolute paths, `..`, symlinks, special
files, generated directories, and common secret paths are excluded from the guided
file/index tools. There is no claim that an explicit Full access terminal is
confined by those rules. Guided text files are limited to 1 MiB; source snapshots
are limited to 4,096 files/64 MiB. Choose a narrower project folder for this workflow,
or use Full access commands for larger/binary work. Checkpoint storage stops at
128 MiB rather than dropping undo history silently. There is no automatic retention
or encrypted secret store yet.

SQLite uses WAL and FULL synchronization. A project lock serializes Alt actions;
atomic replacement preserves ordinary permission bits. Prepared/applying/applied
and undoing/undone journal states distinguish interrupted writes. Recovery compares
actual file hashes before deciding a state; unknown contents become conflicts.
Existing permissions are preserved, including executable bits. Ownership, ACLs,
extended attributes, directory creation, and arbitrary terminal effects are not
fully reversible. Other programs changing a project simultaneously are detected
when their contents no longer match; there is no universal multi-file transaction.

Identical failed native calls are limited to three attempts; an actual successful
edit resets that budget. Missing required arguments and malformed edits fail
without mutation. Plain file/check text is supplied to models to reduce mistakes
copying escaped JSON. Captured output is bounded at 128 KiB per stream, with a
truncation flag; MCP/UI previews are smaller. Export retains captured evidence,
not output that exceeded the capture limit.

## Checks and isolation

Checks run in a disposable copy of tracked source. Guided mode uses Bubblewrap:
private network/PID/IPC namespaces, a temporary home, restricted read-only system
mounts, an empty `/proc`, and a writable source copy. Project-local node_modules,
.venv and venv directories can be mounted read-only for installed dependencies.
Timeouts, owned process groups, output limits, and per-file/descriptor resource
limits bound ordinary mistakes. This is not a complete resource-quota system or
an adversarial multi-tenant security boundary.

Full access checks use the same source-copy approach with ordinary inherited host
permissions/environment; project-local dependencies are referenced from the real
project. Their external side effects are not isolated. Full access **terminal**
commands run in the real project instead of the copy. This distinction is stated
in approval text and saved evidence.

A result identifies the command, code, source-content/permission digest and mode.
“Passed on current files” means that recorded check passed and the source digest
still matches. It does not prove the whole requested feature works, and dependency,
service, and environment changes are not completely versioned by the source digest.
Missing toolchains/dependencies are reported rather than installed automatically by
the isolated runner. Full access can install/use them normally.

If namespaces are blocked or Bubblewrap is missing, Guided checks record the
failure and do not run. They never silently switch to Full access. The cloud host
blocks user namespace setup; the successful isolation test was run in a disposable
Debian 12 container explicitly configured to permit nested namespaces.

## Durable context

Full conversation events and original tool evidence stay in SQLite. Before each
model turn Alt builds a bounded prompt containing the objective, plan, decisions,
recent attempts/failures, latest check evidence, next action and ranked FTS5 file
excerpts. User project decisions carry across conversations. The text index is
rebuilt from current files before retrieval; deleted/changed text is not reused.
Function/symbol names are searchable lexically; this is not an AST index.
Current computed check status and the latest actual evidence receive priority in
the bounded memory. Historical events carry sequence numbers and appear oldest
to newest, so a reproduced failure is distinguishable from a later passing check.
Older evidence stays in storage even when it cannot fit in the current prompt.

Alt closes the previous Goose session and starts a fresh engine context each turn,
while keeping the same visible Alt conversation/task. This compacts working state
without another model and avoids reloading legacy unrestricted tools. It is a
structured-memory strategy, not a lossless replay of every earlier conversation
sentence. Important requirements belong in Remember or the project brief. The
prompt budget uses bounded text heuristics; native token usage still depends on the
selected model/tokenizer and is displayed when ACP reports it. A long project
history does not expand the checkpoint's native context window.

## Hardware and model capability

Settings offers **Fit Alt to this computer**: RAM (including a cgroup v2 limit when
present), CPU features/threads, NVIDIA driver/free VRAM/compute information where
available, isolation availability, and 4K/8K/16K starting choices. File-size budgets
are estimates, not a guarantee of fit. Managed loading checks minimal RAM headroom
and reports allocation/runtime failures; it never silently changes models/settings.
The managed backend stays CPU-compatible. GPU acceleration uses an existing server;
a GTX 1070 still requires real Pascal/CUDA-12-compatible testing.

**Check this model's tool use** / `alt evaluate` runs a two-step native arithmetic
and unique-receipt probe, saves exact responses and selected artifact/configuration,
and distinguishes a valid tool call from using its actual result. It only runs when
the selected model is explicitly marked uncensored/abliterated. Settings can record
that publisher/user claim for an existing external connection. Cancellation stops
the request and owned local runtime. A probe pass is not broad coding certification.

## Packaging and reproduction

`scripts/build-portable.sh` uses a pinned Debian 11 build image and the pinned Rust
1.99 toolchain. The generated package records the binary's measured GLIBC requirement
and checksum in PLATFORM.json. The managed Goose binary requires glibc 2.28; the
managed llama.cpp binary and its shared libraries require glibc 2.34. Alt checks
component ABI requirements before downloading. Older systems can use an existing
compatible runtime or connect to another machine. There is no Windows/macOS release
claim or automatic GPU installer.

Validation commands:

```sh
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
python3 scripts/smoke_tui.py
python3 scripts/smoke_task_tui.py
python3 scripts/smoke_goose.py --goose /path/to/goose
python3 scripts/smoke_goose.py --goose /path/to/goose --provider ollama
python3 scripts/smoke_isolation.py --alt /path/to/alt  # needs working namespaces
bash scripts/build-portable.sh
```

Live evaluation remains opt-in: `scripts/smoke_live_controlled.py --help` and
`alt evaluate`. All observed model failures are retained in docs/evidence/v3.
