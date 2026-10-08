# Alt workspace guide

Type **`alt`** after installation to open the workspace. Start without a model to
explore Home or a practice project. **Ctrl+P** finds actions and **Ctrl+Q** exits.
[See themes, layouts and the current interface](APPEARANCE.md), or the
[launch walkthrough](UX_FINALIZATION.md).

Use the theme control at the top right or **Settings → Appearance** to choose
among six themes, enter your own accent color, change the navigation layout, or
turn decorative graphics off. Changes save automatically. Click the page name at
the top left to find another page, including when using the Focus layout.

The current source adds [small-model controls and workflows](SMALL_MODEL_USAGE.md).
That guide covers the expanded settings menu, skills, native probe and candidates.

> 0.5 adds [check contracts](VERIFICATION_CONTRACTS.md),
> [qualification and measured guidance](QUALIFICATION.md), and selectable tool focus
> in Settings. Project reads/edits, evidence and directory browsing run in background
> workers; Esc requests cancellation, and open drafts survive failed operations.
> Runtime settings now save atomically inside preferences.toml; legacy runtime.toml
> is imported when no inline runtime section exists. Back up state before upgrading:
> binary rollback needs its matching older state backup.

Alt opens without a model. Choose a folder, connect your own model or import/download
one, and install the agent engine when Home offers it. Saved conversations, files,
checks and recovery tools remain available while a model server is offline.

## Your first project

1. Choose **Home → Connect your first model**. Ollama uses a server URL such as
   `http://127.0.0.1:11434`; compatible servers usually end in `/v1`.
   Test the connection, select a listed model, and save. Existing GGUF files can
   be imported from Models; importing preserves the original file.
2. Choose **Home → Choose a project folder**. Start with a small folder you
   recognize, or use **Try a practice project** to explore before connecting.
3. Describe the outcome you want, or pick a Home task starter. Say what successful
   behavior should look like, including what should keep working.
4. Choose your access mode in Settings. **Full access** enables arbitrary commands,
   networking, package installation, jobs, scanners, language servers and MCP tools
   with your normal OS permissions. **Guided changes** supports reviewed file edits
   and isolated checks. **Review only** supports inspection. Access is your choice;
   refusal-related properties come from the model you selected.
5. Review proposed actions. The approval dialog initially selects Reject; choose
   Allow for one action, or explicitly allow the connected session. Commands and
   complete tool arguments are shown. Ctrl+Y approves; ordinary `y` does not.
6. For code changes, open **Task**. Configure a check for the actual requested behavior, mark it
   required, and use **Run required checks**. Multiple requirements are independent:
   one passing check does not certify the whole project. Checks are optional for
   beginning a conversation or asking questions.
7. Review changes and evidence. A green result means the configured checks passed
   on current files, commands and recorded environment. It does not prove untested
   behavior. Model prose never changes computed verification status.

Esc stops active model work or requests cancellation of a background operation.
Atomic storage work completes before stopping. Ctrl+Q exits and restores terminal
settings. Unsent prompts survive connection errors. Close an error to return to a
preserved form; its Next step points to the appropriate recovery controls.

## Pages and controls

| Page | What you can do |
|---|---|
| Home | Connect a model, choose a folder, install engine, use task starters |
| Workspace | Chat, inspect tool requests/results, approve, stop and reconnect |
| Models | Import GGUF, inspect a server's models, search/open Hub repositories, download/resume |
| Connections | Add, test, edit and select model endpoints |
| Conversations | Search, rename, resume, archive/restore and export saved conversations |
| Settings | Themes/layouts/graphics, access/context, tool paths, workflows/MCP, storage, model cache, runtime tuning, benchmark |
| Help | Keyboard shortcuts and usage guidance |
| Task | Objective, plan, evidence, configured/required checks, decisions, diffs and undo |
| Files | Browse paths, filter names, view/search/edit/create text, grouped change history |
| Jobs | Start, attach/detach, stop/restart, view output and persistent job state |
| Context | Inspect the last assembled memory and brief; pin/remove durable requirements |

Alt+1 through Alt+9 open the first nine pages; Alt+0 opens Jobs. Ctrl+P lists every
page, including Context. Narrow terminals show a window of readable navigation
labels; all pages remain keyboard-accessible. Tab focuses navigation. F1 opens Help.

In **Files**, use arrows and Enter to view, `/` to filter names, `S` to find text
in the selected file and `G` to go to a line. Views show up to 300 lines / 24,000
characters per position; the find result includes line numbers. `E` edits, `N`
creates and Ctrl+S previews the change before saving. The built-in editor supports
64 KiB UTF-8 files; larger/native editors work through the Full access terminal.
`D` shows grouped recorded diffs. Checkpoint undo restores only Alt-recorded file
edits and refuses to overwrite conflicting manual changes. It cannot undo arbitrary
terminal commands, databases, installations or remote effects.

## Terminal jobs

From **Jobs**, press `N`, enter a command, choose its lifetime and time budget (0 means unlimited). A command can be
an interactive shell, development server, build, download or test. Alt-owned jobs
stop when their owner exits, including an abrupt exit. Jobs explicitly kept running
use a separate supervisor and can be reattached after reopening Alt. Saved PID,
boot ID and process start identity are checked before signalling a process.

Enter attaches keyboard input; **Ctrl+] detaches**. While attached, Ctrl+C belongs
to the program. Alt forwards paste, cursor/control/function keys and terminal
resizes. Detached `S` stops, `R` restarts and `H` checks a health URL for the selected job. Logs are bounded to
8 MiB with truncation recorded; the screen/tail remains bounded and updates live.
Input has a bounded queue so a program that stops reading cannot freeze Stop.

```bash
alt --access trusted jobs start 'npm run dev' --name 'App server'
alt jobs list
alt jobs health JOB_ID http://127.0.0.1:3000/
alt --access trusted jobs attach JOB_ID
alt jobs stop JOB_ID
# --wait ties the command to the waiting Alt process. --timeout 0 means unlimited.
alt --access trusted jobs start 'cargo test' --wait --timeout 300
```

Use `alt plain` for a plain terminal conversation with per-action approval and
`/quit` to leave. Use `alt run ... --json` for scripts. Headless tool permission is
denied unless `--allow-tools` is supplied; Full access is separately selected with
`--access trusted`. JSON includes verification independently of the model answer.

## Verification, memory and context

Configure checks in Task using the project's documented test command. Mark the
checks that cover your outcome as required. Run them directly without asking the
model to repeat a command. Failed, cancelled, timed out, unrun, zero-test and stale
results remain distinct. Declare the check purpose in Task. Tests contracts need
a complete structured JSON/JUnit/TAP report; command-only results retain unknown
behavioral coverage. Changes to source, check arguments, relevant
lockfiles, executable identity or recorded environment invalidate current results.
Observed tool versions are recorded where available. Snapshots omit generated
folders and credential files such as `.env*`, `.pem` and `.key`; those contents are
not placed in model memory or checkpoint history. Rerun checks after changing
excluded configuration, remote services or other dependencies outside the recorded
fingerprint. A source snapshot is not a complete capture of the machine.

Version 0.5 records explicit contracts and pinned assertions and marks older check
evidence stale. Rerun required checks after upgrading. Complete structured reports
and independently held assertions are described in [verification contracts](VERIFICATION_CONTRACTS.md).
A successful custom command alone does not establish the requested behavior.

```bash
alt task configure-check build -- cargo build --locked
alt task contract build --kind build
alt task require builds --check build --description 'The project builds successfully'
alt --access trusted task verify --run
alt task status
alt task pin 'Keep the public API backward compatible.'
alt task map
alt task index
alt task context
```

The incremental index explores up to 50,000 paths, with 4 MiB per text file and a
256 MiB text budget. Hashes change when file metadata changes; deleted files leave
no stale search excerpts. A fresh reconciliation on search avoids dependence on a
filesystem watcher. Rust, Python and JavaScript use Tree-sitter chunks and symbols.
Editing/check snapshots retain stricter bounds (4,096 files, 64 MiB total, 1 MiB per
file) so large repositories remain searchable even when they exceed mutation limits.

Context shows the actual last memory block, pinned requirements, current check
evidence and retrieved excerpts/decisions. This is persistent retrieval, not a
larger native model context. Managed llama.cpp uses `/tokenize` for the memory
block when available; otherwise Alt reports estimates. The engine budgets the
remaining conversation, instructions and tool schemas separately. A 16K setting
does not guarantee a model or runtime supports an effective 16K conversation.
For Ollama, Alt chat uses the server’s OpenAI-compatible route; choose a tag whose
Modelfile `PARAMETER num_ctx` matches your intended native window. The Alt context
setting controls the agent budget and does not rewrite an external server’s tags.
The native benchmark API sets its own per-request window and reports that scope.

## Workflows, extensions and findings

Open **Settings → Tools and workflows → Project workflows**. The versioned packs
provide labelled inputs, prerequisites, execution records and parsers:

| Pack | Purpose and prerequisite |
|---|---|
| Build | Existing Rust/Python/JavaScript test runner; no hidden dependency installation |
| Git | Status/diff, chosen branch, stage selected paths and commit only chosen paths |
| HTTP | Check expected status/body against a user-specified application URL |
| Browser | Playwright click/visible-text assertions and screenshots; Node, Playwright and browser required |
| Semgrep | Bundled Python application-review rules or supplied config; Semgrep required |
| pip-audit | Audit a Python requirements file; pip-audit in the selected Python environment |
| npm audit | Audit a project lockfile; npm and registry/advisory access required |

Heavy browser/scanner dependencies are optional and are not bundled. Use Jobs to
install the project's documented tools in an appropriate environment. A missing
program, malformed response, zero scanned files/dependencies or disconnected server
cannot produce a passing workflow. Browser/scanner findings are observations to
review, not proof of exploitability. Full access remains available for additional
tools or commands outside the packs.

Findings contain rule/title, severity/confidence, source location where applicable,
real evidence record, reproduction input, proposed fix and linked retests. Settings
can inspect, retest and export them as JSON, Markdown or SARIF. A retest records a
new execution using the original workflow inputs; it does not rewrite old evidence.

**External MCP connections** supports stdio commands and Streamable HTTP with
JSON/SSE responses and session IDs. Add a connection, probe it, and select the
small tool inventory to expose. Credentials are environment references. Stdio
inherits your launch environment; remote bearer tokens stay on the configured
origin. Configuration changes apply on reconnect. Connections recover after errors
on the next call; possibly completed actions are never automatically replayed.
Old standalone HTTP+SSE transport is not implemented.

**Language-server references or rename** uses an installed server such as
rust-analyzer. Supply the project-relative file and 1-based line/UTF-16 column.
Blank new name finds references. A rename presents grouped diffs, checks source
freshness, and uses Alt checkpoints on application. File creation/deletion and
moves in server-provided edits require separate handling. No server is installed
or allowed to apply unreviewed edits automatically.

## Models and hardware

For a GTX 1070 with 8 GiB VRAM and 16 GiB RAM, start by measuring a quantized 3B–4B
model at 4K–8K context. This is a starting configuration, not a promised fit or speed.
The downloaded CPU runtime works without CUDA. An external runtime or a user-selected
GPU build can use acceleration. **Settings → Model and runtime settings** exposes
GPU layers (0 CPU, -1 all), threads (0 automatic), batch size, K/V cache precision
and flash attention. Quantized V cache requires compatible flash attention.
Requested GPU operation fails clearly when the runtime discovers no accelerator.
Container RAM guidance accounts for reclaimable inactive file cache while retaining
anonymous memory, tmpfs and active-file working-set costs. Estimates still do not
guarantee that a context/model allocation will fit.

`scripts/build-pascal-runtime.sh` pins llama.cpp b11429 and requires an installed
CUDA 12 toolkit for architecture 6.1. It is an opt-in build recipe, not a measured
GTX 1070 result. Select its produced server in Settings and benchmark on the actual
computer. Default managed layers remain 0; tuning never substitutes another model.

**Benchmark selected model** records load/integrity time, three short generation
trials, provider token usage, observed runtime RSS/VRAM where accessible and exact
settings. External runtime memory is not claimed as measured by Alt. Failed loads,
allocation errors and cancellation produce reports and stop owned runtimes. Coding
quality is evaluated separately by `scripts/live_acceptance.py` with independent
Python/Rust/JavaScript outcomes. Every real evaluation explicitly requires an
uncensored/abliterated artifact. Publisher labels do not guarantee capability.

Hub downloads pin revision and SHA256. Multipart sets promote only after every
part verifies; retry resumes completed/partial parts. Disk space is checked for
the whole remaining set. `HF_TOKEN` enables access to repositories the account can
already use; the token is not saved in profiles. Actual credentialed private-Hub
acceptance was unavailable in this cloud. See Models' authentication help.

**Settings → Model files and cache** relocates managed weights by copy/verify/switch,
leaving old copies and a receipt for review. Remove a library model to delete its
active managed weights, after removing referencing connections. Imported originals
are never deleted. Retained old relocation copies require explicit manual cleanup
using the receipt; Alt does not guess whether another application needs them.

## Storage, recovery and updates

Settings' **Storage and recovery** shows usage, consistent SQLite backups,
new-directory restore, redacted diagnostics, archived-conversation retention and
old-report cleanup. Diagnostics omit credential values, endpoint URLs, prompts,
source files, project paths and raw logs. Review the exported content before sharing.
Other evidence exports and backups may include project data and are not encrypted.

A core backup includes Alt configuration, conversations, project journals/checkpoints,
execution recovery receipts/logs and inference cost/integrity receipts. Raw inference
requests/responses and inference archives are exported separately; the backup
manifest explicitly records these exclusions. It also excludes model blobs/imported
originals, runtimes, engine cache and live job resources. It is not a filesystem backup of the working project.
Restore verifies a checksummed manifest and requires a new or empty destination.
Model records retain original paths; relocate or download again if those paths are
missing. Database migrations create recovery copies before transactional changes. A shared state-generation barrier coordinates writes; backups briefly take its exclusive side while freezing files and SQLite images, then release it before compression. Active checks or edits produce a retry message if a consistent snapshot cannot be acquired.

Conversation retention first creates a compressed recovery archive, then removes
only old conversations you already marked archived. Unarchived/recent conversations
remain. Restore an archive without overwriting an existing conversation. Report
retention previews eligible exports/evaluation files, finished jobs and inactive inference connections separately. Inference payloads are exported with a hash manifest before removal, while settings, cost/integrity receipts and archive indexes remain. Active leases and legacy connections without leases are protected. Verified inference archives remain under `inference-archives`; preserve them with your backups or move them to another storage location. They are never automatically deleted.

Storage usage reports an explicit inference evidence warning budget (512 MiB by default), current bytes and a recovery action. The warning never cuts off inference or removes files. Checkpoints and databases are preserved. Interrupted file writes and undo are
reconciled against before/after hashes; conflicts remain visible for manual review. Package updates stage and verify complete executable/documentation generations before switching one atomic pointer. Use `bash install.sh --status` to inspect the actual generation after interruption, and `--rollback` to switch to the retained prior executable/documentation. State downgrade still requires the matching pre-upgrade backup in a new folder.

```bash
alt state usage
alt state backup /outside/state/alt-backup.tar.gz
alt state restore /outside/state/alt-backup.tar.gz /new/alt-state
alt state history --days 90          # preview
alt state history --days 90 --apply # archives before removal
alt state restore-history /path/to/history.jsonl.gz
alt state diagnostics > diagnostics.json
```

The local Linux package is not a published release. Its archive has a SHA256 sidecar,
all-file manifest, dependency notices, source hashes and measured ABI metadata.
Checksums establish consistency, not publisher identity. If a trusted release key
is configured at build time (`ALT_SIGNING_KEY`), verify the downloaded archive
**before extracting/running its installer**:

```bash
openssl dgst -sha256 -verify trusted-release-public.pem \
  -signature alt-0.4.1-linux-x86_64.tar.gz.sig alt-0.4.1-linux-x86_64.tar.gz
# Then extract and optionally have the installer also verify the contents signature:
ALT_PREFIX="$HOME/.local" ./install.sh --verify-key /path/to/trusted-release-public.pem
# For the local unsigned build, run ./install.sh without --verify-key.
./install.sh --rollback
```

Installation atomically replaces the executable, retains the previous distinct
binary, and syncs filesystem changes. Reinstalling the same build does not consume
the rollback copy. State stays separate. The 0.3 upgrade/rollback fixture checks
compatible preferences/project evidence; newer verification metadata is ignored by
0.3 and recomputed after returning to 0.4. Downgrading arbitrary future schemas is
not guaranteed. Reproducible archive metadata is checked separately from compiler
bit-for-bit reproducibility, which is not claimed across different hosts/toolchains.
