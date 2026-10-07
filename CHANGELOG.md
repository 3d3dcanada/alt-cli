# Changelog

## 0.6.0 beta

- Added a four-case practice project with independent checks, tracked repair and undo.
- Matched model instructions to selected tools and configured checks; surfaced evidence errors as failed calls.
- Improved check discovery, small-terminal evidence display and bounded retrieval.
- Fixed partial-download write completion and Goose synthetic network-error handling.
- Added sustained stream recovery, complete model matrix tooling, automatic pre-upgrade backups, GitHub-attested release publication and a local PC test kit.


## 0.5.0 beta — 2026-10-06

First full source publication. Earlier 0.2–0.4.1 versions in the research/evidence
folders were development iterations; their presence does not imply public releases.

### Workspace and tools

- Rust CLI and eleven-page TUI with guided setup, project browsing/editing, diffs,
  checkpoint undo, saved/resumable conversations and retained drafts after errors.
- Interactive PTY jobs with input, resize, attach/detach, stop/restart and health
  checks; explicit Full access supports arbitrary host commands and networking.
- Native project tools, optional workflow packs, selected external MCP tools,
  language-server references/rename, persistent goals and retrieved project context.
- Compatible model-server connections, verified/resumable Hub downloads, split
  GGUF support and safe references to imported model files.

### Audit follow-up

- Explicit tests/build/lint/health/custom contracts, bounded JSON/JUnit/TAP reports,
  independently pinned assertions, dependency freshness and separate command/
  required-check/behavioral outcomes.
- Background project and conversation operations, cooperative cancellation,
  generation-tagged results, coalesced refreshes and form preservation.
- Selectable tool focus, actual check feedback, explicit reasoning controls,
  exact configuration identity and resumable independent model evaluations.
- CPU/GPU runtime controls and generation/context/cancellation/restart qualification,
  including actionable memory and missing-library errors without model substitution.
- Compile-time provenance, stale-binary rejection, dependency SBOM, portable build,
  recovery/failure injection, bounded parser mutation and terminal resource checks.

### Validation and limits

79 Rust tests and four terminal walkthroughs passed locally. Thirty responsiveness
journeys, thirty PTY lifecycle rounds and real package install/state rollback
checks on Debian 11 and Ubuntu 24.04 also passed. Retained uncensored-model CPU
qualification reaches 16K context; physical GTX 1070 performance is unmeasured.

All three latest Spark coding pilots failed 0/5. Full development/held-out matrices,
real LM Studio/private-Hub qualification, actual novice sessions and production
signing remain open gates. See [the report](docs/IMPLEMENTATION.md) for exact scope,
raw positive/negative evidence and historical build identities.
