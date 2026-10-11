# Changelog

## Unreleased — 0.7 (work orders in docs/WORK_ORDERS_0.7.md)

### A05 model capability probe

- *Probe this model* (Settings → Models, Ctrl+P, `alt probe`) reads native context
  and template facts (`/props` on llama.cpp, `/api/show` on Ollama), then sends one
  bounded request (at most 512 output tokens) with a trivial tool and the model's
  default thinking. It records whether the model called the tool, reasoned, or hit
  the limit.
- Suggestions are labelled choices: thinking off or low, a larger output allowance,
  compact tools, or matching the server's native context. Nothing is applied until
  you pick one (`alt probe --apply ID` headless). Reports are stored per model
  identity under `probes/`. Owned runtimes are started and stopped for the probe.

### A04 hosted-provider resilience

- Retries for HTTP 429, 5xx and dropped connections with capped exponential
  backoff that honours `Retry-After` (default 2, 0–8). Each retry counts as a model
  call against the shared allowance and is logged in the receipt.
- Output guard (automatic for remote servers, on or off): the relay holds each
  response, detects leaked chat-template tokens (`<|…|>`) or mixed-script noise in
  scripts the conversation never used, discards it once and resends. The discarded
  bytes are kept as evidence and recorded as a provider fault, never as model
  output; a second fault returns a clear error. Tokens stay charged.
- Model output and sampling → *Provider retries* and *Output guard*, Ctrl+P →
  *Hosted output guard*, and `alt inference --retries N --output-guard auto|on|off`.

### A03 provider presets and hosted discovery (+ B03 authenticated Ollama)

- Presets fill the endpoint, key-variable name and thinking dialect: NVIDIA NIM,
  OpenRouter, Groq, Together, DeepSeek, Mistral, Gemini (OpenAI-compatible),
  Cerebras, LM Studio, vLLM/SGLang, llama.cpp, Ollama and Custom. TUI: Connect →
  *A hosted API or another server preset*, or Ctrl+P → *Connect a hosted API*.
  Headless: `alt init --preset nvidia --model z-ai/glm-5.3`.
- The model picker searches long hosted catalogs as you type; `alt models
  --filter WORDS` does the same headless.
- Keys remain environment-variable references; a missing variable is reported
  before connecting. `smoke_goose` proves the key reaches the provider on every
  model call and never appears in Alt's state, logs or evidence.
- B03: Ollama connections with a key variable work end to end (the engine no
  longer refuses them; the relay already sends the bearer key).

### A02 open parameter surface

- Request parameters per connection: repeat penalty (spelled per server),
  presence/frequency penalty, seed, stop sequences, typical-p, and an extra
  request JSON object merged last. Alt-owned keys (`model`, `messages`, `tools`,
  `tool_choice`, `stream`, `stream_options`, output-token fields, `n`) are refused.
- Ollama receives `options.num_ctx` from the connection context and an optional
  `keep_alive` unless *server default* is chosen; Alt reads the loaded context back
  from `/api/ps` and says when the server ignored it.
- Owned llama-server: mmproj, MoE-on-CPU layers, tensor override and split, main
  GPU, RoPE/YaRN scaling, mlock, no-mmap, chat-template file, draft model for
  speculative decoding, and `extra_args` (binding, key, model, alias and context
  flags refused). The argv comes from one pure function (`runtime::server_args`).
- TUI: Settings → Models → *Advanced parameters* and Ctrl+P, with labelled fields
  and a raw JSON view of the exact request fields and runtime argv. CLI:
  `alt params set|unset|show`.

### A01 reasoning control and thinking-cutoff recovery

- One **Thinking** setting (`default | off | low | medium | high | budget:N`),
  translated per server dialect: llama.cpp `chat_template_kwargs.enable_thinking` +
  `reasoning_budget_tokens`; Ollama `think` + `reasoning_effort`; vLLM / SGLang /
  NIM `chat_template_kwargs`; OpenAI-style `reasoning_effort`. Receipts record the
  fields sent. Settings → Models, Ctrl+P, and `alt inference --thinking/--dialect`.
- A response that ends at the output limit with reasoning only is a *thinking
  cutoff*: charged, never resent silently. The TUI offers labelled recovery
  (thinking off or low for this connection, or a larger allowance); headless
  `--json` emits a typed `reasoning_cutoff` event.

### A07 security (audit 2026-10-08)

- "Allow this session" and `--allow-tools` now cover project actions only
  (checkpointed edits, configured checks, skill helpers). Terminal commands,
  external MCP tools and untagged engine requests need a separate opt-in:
  **Allow terminal this session** in the TUI or `--allow-terminal` headless.
  Candidate exploration keeps `--allow-tools` only, so its terminal requests are denied.
- Project tools never list, read or edit Alt's own data folder when a project
  contains it; choosing `$HOME` or `/` as the project shows a warning.
- More credential names are excluded and compared case-insensitively: `.envrc`,
  `.netrc`, `.npmrc`, `.pypirc`, `.git-credentials`, `id_rsa*`, `id_ed25519*`,
  `id_ecdsa*`, `credentials*.json`, `.docker/`, `.kube/`, `.gnupg/`, `*.p12`, `*.pfx`.
- Remote MCP connections with an authentication variable require HTTPS unless
  the server is on loopback.
- The owned llama-server starts with a random per-launch key passed through a
  private `--api-key-file`, and readiness requires an authenticated model listing.
- Headless `extensions check` refuses to start the server unless access is Full
  (same rule as the TUI).
- `requirements-dev.txt` is hash-pinned and installed with `--require-hashes`; the
  release job checks out without persisted credentials; SECURITY.md names the
  current line and the owner's private-reporting switch.

## Unreleased — Neon interface (work orders N01–N09)

- Model lists now show distinct descriptions per row: parameters, quantization,
  file size, family and modified date where the server provides them (Ollama
  `/api/tags`), with a curated family blurb in the picker and an honest
  "no details reported" fallback. Duplicate-looking rows are gone.
- Launch is never a dead end: a missing saved project folder opens an
  "Open a project" chooser (current folder, recent projects, browse, create new,
  practice sandbox), and dismissing it opens Alt's sandbox workspace so every
  page works. Recent projects are recorded and offered.
- Theme engine with five selectable looks (Neon Grid default, Amber CRT, IRIS,
  Bleached, Stock): every color, border style and glyph is data; the choice
  persists and redraws live. Status header gains a connection LED, access badge,
  animated busy spinner and a context-budget meter.
- Settings reorganized into five tabs (Appearance, Models, Tools & access,
  Runtime, Data) with ←→ switching; a regression test proves every setting sits
  in exactly one tab.
- Ctrl+P is now a fuzzy command palette: every page, setting and manager action,
  typed filtering, keyboard and mouse.
- Bundled skills can be switched on/off from Tools and workflows; disabled
  skills are excluded from the model's list/read/helper paths and cannot stay
  the active procedure. Nothing auto-enables.
- Layout modes: sidebar, top tabs, bottom tabs and a wide split view with a
  live activity column; Ctrl+L cycles and the choice persists.
- Verification: strict Clippy and fmt clean; serial unit tests green; the full
  first-run PTY battery (`smoke_tui.py`) and `smoke_task_tui.py` pass.
  Three timing-budget recovery tests flake under host load above ~10; the same
  flakes reproduce on the pre-change baseline, and they pass serially.

## 0.6.0 beta 3 — corrected release validation

- Handle a child exiting during a procfs status/identity read in terminal and
  streamed-recovery cleanup checks; unexpected I/O errors still fail validation.
- Add five regression checks for vanished processes, zombies, live children, PID
  reuse and unexpected read errors, including an actual child-process lifecycle.
- Require successful exact-tag Verify CI before publishing, with five gate
  regressions and an attached verification receipt.
- Retain the failed beta 2 tag-verification log and leave that unpublished tag
  unchanged. Update downloads to this corrected candidate. Application behavior
  and the previously recorded live-model outcomes are unchanged.

## 0.6.0 beta 2 — small-model harness and PC completion

- Added a disposable PC recorder for actual repair, independent four-case
  acceptance, goal-preserving follow-up, cancellation/resume and undo. Reuses
  the selected model without changing normal configuration or copying weights.
- Added observed inference stages and remaining allowances in Chat, confirmed
  finite allowance additions, and explicit allocation updates for saved tasks.
  Spent and interrupted request costs remain recorded.
- Added integrity-checked failed-case feedback, source diagnostic handles and
  advisory recovery for unchanged failures and revisited source revisions.
- Tested both adapters with twelve-turn conversations in Git projects retaining
  uncommitted user changes, external-edit recovery and sixty stream scenarios.
- Show the exact beta tag or source identity in the TUI header and build metadata.
- Updated installation, model compatibility and local qualification instructions.
- Added language-tool preflight so a broken Rust/Node environment stops a live
  evaluation before model requests and remains distinct from incorrect source.

- Added explicit compact text/line-array tool focus in CLI/TUI, short
  revision-bound handles, required edit fields, checkpointed creation/deletion,
  complete current-source packets and refreshed handles after edits.
- Preserve the complete original request on compact follow-ups, with the latest
  user message taking precedence; exercise two turns in the actual persistent
  CLI/TUI conversation and refuse over-budget scope rather than truncate it.
- Added missing-definition/parser observations and disposable-check path
  guidance; classify Goose synthetic provider/allowance failures as failed turns
  while retaining source and conversation evidence.
- Retained a complete MiMo 9B Q4 CPU pilot (3/4 independent passes per interface)
  with smaller compact input and faster passing repairs. The subsequent build
  passed 6/9 MiMo attempts, including 2/4 held-out tasks and the actual 80×24 TUI
  repair. Retained protocol journeys and two pending-review training captures.
  Old failures remain unchanged; longer CPU limits are not a like-for-like gain.

- Added a separately versioned v5 behavioral suite, new sealed families and
  immutable matched campaign schedules without rewriting historical oracles.
- Added migrated explicit inference settings, actual provider request/response
  receipts, full owned chat/tokenizer accounting when available and shared
  generation/request limits with conservative interrupted costs.
- Added revision-bound range/symbol edits, multi-file failure rollback, explicit
  host workflows, compact actual diagnostics, structural retrieval and six
  versioned skills backed by executable helpers.
- Added streamed native-tool round trips, serial source candidates with shared
  effort/review/apply/undo, offline instruction drafts with separate validation
  and reviewed activation/rollback, and labelled TUI choices for these workflows.
- Added pending-review trace capture, native argument validation, SFT checkpoint
  resume integrity, exact-parent merge/Q4 export preparation and verified rewards.
- Added bounded recursive analysis and source-state adapter qualification as
  explicit experiments. Retained 7B native failures and a MiMo 9B CPU native pass
  consuming a fresh, unpredictable host result. Retained all 22 failed coding
  pilot attempts; no quality preset or training trajectory is promoted.
  GPU training, physical GTX 1070 qualification and repeated quality studies
  remain unperformed/unpromoted; see `docs/SMALL_MODEL_DELIVERY.md`.

## 0.6.0 beta

- Added a four-case practice project with independent checks, tracked repair and undo.
- Matched model instructions to selected tools and configured checks; surfaced evidence errors as failed calls.
- Improved check discovery, small-terminal evidence display and bounded retrieval.
- Fixed partial-download write completion and Goose synthetic network-error handling.
- Fixed file-edit submission racing background listings, retained changed filenames on narrow screens, and preserved actionable missing-interpreter errors.
- Removed search highlighting from source snippets and strengthened evaluation completion/snapshot checks.
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
