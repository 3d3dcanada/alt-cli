# Implementation and verification — Alt 0.3.0

October 6, 2026. The six approved additions are implemented in the Rust CLI/TUI.
[Controlled workflows](CONTROLLED_WORKFLOWS.md) describes their behavior and limits.
The [0.2 record](IMPLEMENTATION_0.2.md) is preserved as history; its four unrestricted
Goose tools and absence of project checkpoints/retrieval are superseded.

| Area | Implementation | Verification |
|---|---|---|
| Controlled tools and Full access | Alt-owned MCP broker; required reads/plans, exact edits, argument checks, one-use approvals, duplicate failure budget; explicit unrestricted terminal mode | Native bridge tests; real Goose/OpenAI and Goose/Ollama fixtures; denial and approved effects |
| Checkpoints, diffs and undo | SQLite action journal, original/postimage contents, atomic writes, file permissions, reverse task undo, conflict/crash detection | Stale reads/approvals, create/delete, pre-existing manual work, conflict preflight and interrupted journal tests |
| Guided task workflow | Inspect/Plan/Edit/Test/Results, evidence/next action panel, direct configured check reruns, test-change notices, exports | Real PTY task workflow; independent file/command checks |
| Durable memory and context | Goals, plans, user decisions, attempted actions/failures, evidence references; FTS5 current-file retrieval; fresh bounded engine context each turn | Index invalidation, restart/resume, preserved notes/evidence, unchanged provider/model fixture assertions |
| Hardware and model guidance | RAM/cgroup/CPU/GPU/driver inspection, context presets, minimum RAM/ABI checks, explicit uncensored native-tool capability probe | Hardware/ABI inspection and model/protocol evaluations; no GTX 1070 measurement |
| Linux packaging and isolation | Debian 11 build baseline; measured GLIBC metadata; component preflight; Bubblewrap isolated checks and explicit Full access | Older-container binary checks; Debian 12 engine/runtime chain; isolated network/filesystem test; cloud blocked-isolation case |

The app has eight pages: Home, Workspace, Models, Connections, Conversations,
Settings, Help and Task progress. Settings includes access choices, hardware
profiles, capability evaluation and publisher/user uncensored-claim recording.
Files/notes/evidence can be reviewed without a working model connection.

The model-facing tools are `list`, `read`, `search`, `remember`, `edit`, `run_check`,
and `terminal`. `terminal` is authorized only in Full access; the model cannot
change the selected policy. Model notes cannot certify a check as passed.

## Validation and evidence

All **21 Rust tests passed**, along with formatting, strict Clippy, both real-Goose
provider fixtures, and the two real-terminal acceptance scripts. Run
`bash scripts/setup-cloud.sh` for the normal cloud checks. The test suite now
covers the project journal, native tool broker, check runner and failure handling
as well as existing ACP, Unicode/editor, model download/import and TUI behavior.
Real HTTP/ACP fixtures are deterministic and contain no model weights.
The [machine-readable validation record](evidence/v3/validation.json) identifies
the final binary and separates cloud, container, isolation and package checks.
The package installed twice under a temporary prefix with the same binary bytes,
documentation and dependency licenses. No release was uploaded or published.

Additional acceptance scripts:

- `scripts/smoke_task_tui.py`: no-model task setup, direct reruns, access selection,
  isolation failure reporting, evidence, persistent decisions and memory search.
- `scripts/smoke_isolation.py`: source copy integrity, hidden host file and blocked
  networking on a host that permits namespaces. The cloud itself blocks these;
  the successful test used a disposable namespace-enabled Debian 12 container.
- `scripts/build-portable.sh`: pinned Debian 11 image, pinned Rust 1.99.0,
  local unpublished package with licenses, checksum and PLATFORM.json.
- `scripts/smoke_live_controlled.py`: explicit uncensored artifact, independent
  assertions, real tool/check records, direct rerun and conversation resume.
  `--direct-edit-instructions` is a separately labelled tool-execution fixture;
  it supplies exact edit arguments and is not a bug-reasoning benchmark.

Raw evidence and screenshots are under `docs/evidence/v3` and
`docs/screenshots/v3`. The first two 0.3 Heretic autonomous repair evaluations failed;
the guards preserved the files/assertions and checks stayed failed. After tool
response improvements, the directed edit/check/resume fixture passed with unchanged
assertions and an independently rerun result. A subsequent autonomous repair also
passed without supplied replacement text. The native two-step capability probe
passed. Both original resumed explanations misread the history despite successful
read-only completion; this prompted a memory-ordering/budget fix and regression
test. The repeated natural-language resume correctly reported the latest evidence
and chronology after that fix, preserving files without rerunning checks. See
[live evaluation](LIVE_EVALUATION.md) for all outcomes. These small fixtures do not establish broad
model reliability.

## Practical limits

Native model context and tool quality remain model/template/runtime-dependent.
Guided edits are bounded UTF-8 text operations; Full access supports broader host
operations with their normal effects. Checkpoint undo covers Alt edits, not arbitrary
terminal commands, databases, ACLs, extended attributes, or remote systems.
Snapshots identify source contents and ordinary permissions, not every dependency
or service state. Output beyond capture limits is explicitly truncated.

Managed GPU installation, Pascal benchmarking, Windows/macOS releases, gated Hub
login, sharded model downloads, automatic retention/encryption, and a broad
multi-model quality benchmark are not implemented. Linux x86_64 is the tested
release target. The general adapter supports arbitrary compatible endpoint/model
IDs; that is not a claim that every checkpoint can use tools reliably.
