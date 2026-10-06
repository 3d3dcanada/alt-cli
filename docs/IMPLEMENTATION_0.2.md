# Implementation and verification

Updated October 6, 2026 for **Alt 0.2.0**. This is a substantial Linux development
build. Cross-platform distribution, target-GPU tuning, retrieval/compaction, and broad
model-quality evaluation remain outstanding.

## Product surface

The Ratatui interface has Home, Workspace, Models, Connections, Conversations,
Settings, and Help. First launch needs no configuration. Guided presets, model
selection, folder/file browsing, task templates, quick actions, contextual hints,
and mouse/keyboard input avoid requiring TOML edits. The editor handles grapheme
boundaries, cursor movement, multiline messages, bracketed paste, and wide Unicode.
Layouts are exercised down to 60×18; 100×30 or larger provides more room.

Connection setup tests endpoints and preserves entries after failure. Profiles
accept OpenAI-compatible Chat Completions or Ollama endpoints and arbitrary model
IDs. Keys are environment-variable references. Resume retains original project
and model settings. Authenticated Ollama inference is currently rejected.

Hub discovery reads public metadata over verified HTTPS. Downloads pin revision,
length, and SHA256; validate Content-Range; safely restart if Range is ignored;
check free space; lock concurrent writers; and quarantine checksum failures.
Cancellation preserves partial bytes. Imports hash and reference original files.
Only complete single-file GGUFs are offered; private/gated Hub login is absent.

Pinned Linux x86_64 installers fetch Goose 1.53.0 and llama.cpp b11429. The managed
runtime rechecks model integrity, starts a loopback CPU server on an available
port, waits for readiness, saves logs, and stops its owned process group. Existing
external servers handle GPU acceleration. Alt does not install CUDA or assume VRAM fit.

## Agent, evidence, and context

`engine.rs` owns ACP v1 and Goose-specific options. New sessions explicitly expose
`tree`, `shell`, `edit`, and `write` via `_meta.enabledExtensions`. Shell covers
reads, searches, and checks. Tool-shim inference, model-generated session naming,
and telemetry are disabled. There is no silent fallback model/provider.

The [operator instruction](../prompts/operator.md) and optional project brief are
separate prompt blocks. UI defaults are 8K context and 12 steps. Output is capped
at 2,048 tokens or one quarter of context. The UI displays reported context use,
elapsed time, and tool activity. A budget cannot enlarge native context or configure
external servers automatically. Project notes are explicit memory; automatic
retrieval and tested durable compaction remain future work.

Approval defaults to rejection and shows readable commands/text edits plus complete
request details. One-time and session-only approvals are explicit. A turn with no
tool calls is labelled explicitly, and nonzero shell exits display as failed
activity even if the engine reports that the tool call completed. There is no OS
sandbox, path policy engine, rollback system, or bundled pentest scanner pack.
Working directories are not access boundaries. Read-only classifications come
from Goose and may bypass a permission prompt.

SQLite stores full accepted events, original session settings, titles, archives,
and project briefs. Exports stream JSONL and readable Markdown. Interactive recovery
reads at most 600 recent events/4 MiB; full records remain exportable. Visible chat
is bounded to approximately 200 KB, ACP frames to 1 MiB, and the transport queue
to 128 entries. Display strips terminal control characters. Failed engine sessions
can reconnect. Malformed settings are backed up for recovery; model-record errors
are shown without closing the UI.

Cancellation sends ACP cancel, then kills the owned process group after five seconds
if necessary. Goose 1.53.0 may stall before provider response headers; smoke tests
exercise forced cleanup. The UI remains available to reconnect. Completed effects
are not rolled back. SIGTERM/SIGHUP trigger graceful UI shutdown; on Linux, parent-death
signals stop the direct agent and inference processes after an abrupt parent exit.
This is not a cgroup or a guarantee covering every descendant of an arbitrary
command after SIGKILL. UI tasks have a 20-minute limit; headless mode defaults to ten.

## Verification

| Check | Evidence |
|---|---|
| Rust tests — 14 passing | Unicode editing, responsive layouts, recovery copies, preserved forms, approval selection, settings persistence, session/archive/history bounds, ACP/CLI recovery, cancellation/crashes/malformed frames, download resume/integrity/pause/import |
| Formatting and strict Clippy | `cargo fmt --all --check`; `cargo clippy --locked --all-targets -- -D warnings` |
| First-run PTY acceptance | Guided connection/model setup, folder picker, Unicode/multiline paste, brief delivery, approval rejection, cancellation, search/rename/export/resume, connection repair, import, navigation, resize, terminal restoration, SIGTERM and Linux parent-death cleanup |
| Real Goose with both API protocols | Four-tool inventory, actual denied/approved shell effects, evidence, instructions, session recovery, forced cancellation cleanup |
| Real upstream services | App-native HTTPS Hub search/file metadata, verified installers, GGUF import, managed-runtime doctor |
| Uncensored inference | [Live evaluation](LIVE_EVALUATION.md) records checkpoint, task outcomes, download, process lifecycle, and UI evidence |

Fixtures are not LLMs; they reproduce failure conditions deterministically. Live
inference uses only the publisher-labelled Heretic checkpoint selected for this project.

## Reproducibility and remaining work

`scripts/setup-cloud.sh` installs pinned dependencies, builds, and runs fixtures.
The toolchain file, Cargo lockfile, and artifact SHA256 pins capture tested versions.
Release compilation is also checked. Saved cloud installation/startup instructions
are a draft; saving them does not publish a new environment snapshot.

The cloud machine has about 33 GiB RAM and no NVIDIA GPU. Results do not establish
GTX 1070 performance, compatibility with every old CPU, macOS/Windows support,
or reliability across all models. Priorities are a real GTX 1070 evaluation,
portable release packaging, context/retrieval stress tests, and typed software/security
workflow packs. Spark-X2.5 and MiMo need exact derivative/runtime validation.

## Local package

`python3 scripts/package-linux.py` builds a local `dist/` archive with the release
binary, user installer, documentation, locked dependency provenance, and available
crate/Rust license and notice files. Goose and llama.cpp are separate opt-in downloads.
The development binary built here requires Linux x86_64 and glibc 2.39 or newer;
older distributions need a source build. The installer supports a user-owned
`ALT_PREFIX` and requires no root access. Package SHA256, extraction, repeated
installation into a temporary prefix, installed binary identity, CLI startup, and
included documentation were checked. This artifact has not been published.
