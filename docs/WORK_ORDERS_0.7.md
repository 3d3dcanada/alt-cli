# 0.7 work orders: any model, any server, as strong as possible

Owner direction (2026-10-08): Alt stays public. It must accept and get the best out
of whatever model a user brings: local GGUF, Ollama, LM Studio, llama.cpp, vLLM and
hosted OpenAI-compatible APIs such as NVIDIA NIM (`NVIDIA_API_KEY`). Uncensored,
abliterated and standard checkpoints are all first-class. Evidence:
[2026-10-08 PC model trial](research/2026-10-08-pc-model-trial/README.md).

Rules for every order: keep the evidence discipline (model prose is never the
verification authority; failures are retained). Never silently switch the selected
model, provider or access mode. Every new parameter is an explicit user choice or
a labelled suggestion. Each order ships tests, strict Clippy, fmt and docs. The
cloud has no GPU, no local weights and no API keys: prove behaviour with
deterministic fixture servers (`tests/fixtures/`), not live models. Live
re-measurement (A10) runs on the owner PC.

| Order | Deliverable | Depends |
| --- | --- | --- |
| A01 | Provider-aware reasoning control and reasoning-cutoff recovery | |
| A02 | Open parameter surface for requests and the owned runtime | |
| A03 | Provider presets and hosted model discovery | |
| A04 | Hosted-provider resilience: retries and degenerate-output detection | |
| A05 | Model capability probe with suggested settings | A01, A02 |
| A06 | Small-model edit robustness | |
| A07 | Security fixes from the 2026-10-08 audit | |
| A08 | Managed CUDA 12 runtime option (Pascal sm_61 and newer) | |
| A09 | Repository weight and public-facing README | |
| A10 | Harness labels and matched re-run script (owner PC executes) | A01–A06 |

## A01 Provider-aware reasoning control

Measured: the relay sends only llama.cpp's `reasoning_budget_tokens`. Spark-X2.5 4B
on Ollama with 1,024 output made zero tool calls on 8/8 tasks; every token was
`reasoning`. GLM 5.3 on NVIDIA exhausted 4,096 tokens on 3/8 tasks.

- One setting `thinking = default | off | low | medium | high | budget(N)`, mapped
  per provider: Ollama native `think` and `/v1` `reasoning_effort`; llama.cpp
  `chat_template_kwargs.enable_thinking` + `reasoning_budget`; vLLM / SGLang / NIM
  `chat_template_kwargs` (`enable_thinking`, `thinking`); OpenAI-style
  `reasoning_effort`. The receipt records which fields were sent.
- Detect a response ending at the output limit with reasoning but no content and
  no tool call. Offer a labelled recovery (resend with thinking off, or a larger
  allowance); in headless `--json`, emit a typed event. Never retry silently; the
  spent tokens stay charged.
- Acceptance: fixture asserts the outgoing body for each provider shape; fixture
  reproduces reasoning-only cutoff and the recovery event.

## A02 Open parameter surface

Measured: only temperature/top_p/top_k/min_p reach the server, and
`src/inference.rs` removes `options`, so the profile context never reaches Ollama.
The owned llama-server launch (`src/runtime.rs`) is a fixed argument list.

- Request: repeat/presence/frequency penalty, seed, stop, typical_p, and a
  per-profile `extra_body` JSON object merged last. Refuse keys Alt owns (`model`,
  `messages`, `tools`, `stream`, output-token fields) with a clear message.
- Ollama: send `options.num_ctx` from the profile context and `keep_alive`, unless
  the user picks "server default". Show the effective value read from `/api/ps`.
- Owned runtime: named settings for `--mmproj`, `--n-cpu-moe` / `--override-tensor`,
  `--tensor-split`, `--main-gpu`, rope/YaRN scaling, `--mlock` / `--no-mmap`,
  chat-template file, speculative decoding (draft model or n-gram), plus an
  `extra_args` list. Refuse `--host`, `--port`, `--api-key`, `--alias`.
- TUI: "Advanced parameters" in Alt+6 with labelled fields and a raw JSON view.
  CLI: `alt params set|unset|show`.
- Acceptance: merge-order and refusal unit tests; fixture proves Ollama receives
  `num_ctx`; golden test of the runtime argv.

## A03 Provider presets and hosted discovery

- Presets filling endpoint and key variable name: NVIDIA NIM
  (`https://integrate.api.nvidia.com/v1`, `NVIDIA_API_KEY`), OpenRouter, Groq,
  Together, DeepSeek, Mistral, Gemini OpenAI-compatible, Cerebras, LM Studio, vLLM,
  llama.cpp, Ollama, Custom.
- Model picker from `GET /models` with search. Keys remain env-var references only.
- Acceptance: `alt init --preset nvidia --model z-ai/glm-5.3` works headless;
  test that no key value is ever written to state, logs or evidence.

## A04 Hosted-provider resilience

Measured: NVIDIA `moonshotai/kimi-k3` streamed garbage with leaked template tokens
on 2/8 tasks; replays gave about 1 in 5 garbage at any temperature, plus HTTP 429.

- Retry 429 / 5xx / connection reset with capped exponential backoff honouring
  `Retry-After`; count retries in the receipt.
- Degenerate-output detector (leaked `<|...|>` template tokens, high mixed-script
  ratio versus the conversation). Discard and resend once, recorded as a provider
  fault, never as model output.
- Acceptance: fault-injection fixtures in `tests/faults.rs` for each case.

## A05 Model capability probe

- On connect, a bounded probe: one tool call, thinking detection, native context
  from `/props` (llama.cpp) or `/api/show` (Ollama), output-limit behaviour.
  Suggest settings (thinking mode, output allowance, compact tools) as labelled
  choices; store per model identity. Never apply without the user's choice.
- Acceptance: probe fixtures for llama.cpp, Ollama, OpenAI-compatible.

## A06 Small-model edit robustness

- `replace_file`: handle-bound whole-file rewrite of an already-read file under
  about 300 lines, in the compact profiles. Whole-file edits are the most reliable
  format for models under about 14B.
- `old_text` replace: exact, then whitespace-normalised; on miss return the closest
  current span with its handle.
- Acceptance: unit tests; A10 measures the effect.

## A07 Security fixes (audit 2026-10-08: no critical or high)

1. "Trust session" and `--allow-tools` must not auto-approve `terminal` or external
   extension calls; add a separate explicit opt-in (`--allow-terminal`).
   (`src/tui/app.rs` approval handling, `src/main.rs` headless approval.)
2. Refuse edits under the Alt data directory; warn when the project root is `$HOME`
   or `/` (`src/project.rs` exclusion list, `check_path`).
3. Extend excluded secret names: `.netrc`, `.npmrc`, `.pypirc`, `id_rsa*`,
   `id_ed25519*`, `.envrc`, `credentials*.json`, `.docker/`.
4. Compare excluded names case-insensitively.
5. Plain-HTTP extensions with `auth_env` only on loopback (`src/extensions.rs`).
6. Start the owned llama-server with a random `--api-key` (`src/runtime.rs`).
7. CI: `persist-credentials: false` on write jobs; `--require-hashes` in
   `scripts/setup-cloud.sh`; `SECURITY.md` names the current line; note that
   GitHub private vulnerability reporting must be enabled by the owner.

## A08 Managed CUDA 12 runtime

The managed runtime download is CPU-only. On the reference GTX 1070 a CUDA 12
llama.cpp build ran MiMo 9B Q4 fully offloaded at 16K with q8 KV in about 6.3 GB.
Add a pinned, checksummed CUDA 12 runtime option with GPU detection. CPU stays the
default; never auto-switch. Prove download/verify/launch argv with fixtures.

## A09 Repository weight and README

`docs/` is 176 MB and 9,129 files, mostly raw evidence. New raw evidence goes to
release assets; git keeps summaries. Do not rewrite history. Open the README with
what a new user needs: install, connect a model (local or hosted preset), first task.

## A10 Harness labels and re-run script

- Replace the mandatory `--uncensored` in `scripts/live_acceptance.py` with
  `--artifact-label {uncensored,standard,hosted}`, recorded truthfully; hosted runs
  need no local file hash.
- Add `scripts/pc_matrix_0_7.sh` that runs the research-note matrix (8 cases, 16K,
  compact, 4,096 output, 3 repeats) for Spark 4B (Ollama), MiMo 9B abliterated and
  standard (llama.cpp CUDA), and two NVIDIA models, checking
  `ora-can-i-start`-style GPU headroom first and stopping every server it starts.
  The owner PC runs it; the cloud only verifies it with `--dry-run`.

## Board on main (2026-10-11)

Base: `main` = 0.6.0 + Neon N01–N09 ([NEON_WORK_ORDERS.md](NEON_WORK_ORDERS.md)).
Three unmerged branches were reviewed and harvested by hand, never merged:

- `review/2026-10-08`: this file and the [trial note](research/2026-10-08-pc-model-trial/README.md)
  (brought over unchanged above this section). Its `--api-key-env` harness change is
  carried by A10.
- `audit/final-pass-2026-10-08`: [final audit](FINAL_AUDIT_2026-10-08.md),
  [F01–F14 work orders](FINAL_PASS_WORK_ORDERS.md) and their small bounded
  evidence. Every finding is still open on `main`: the implementation commit on
  the build branch (`bb34872`) was never merged. Open items are numbered B01–B21
  below. Audit finding IDs are written `audit A05` so they do not collide with
  the 0.7 orders A01–A10.
- `build/final-pass-2026-10-08`: saved themes and layouts overlap N03/N08 (main's
  theme engine wins). The graphical Home dashboard is not in main and becomes
  H01. Its F01–F14 implementation (`bb34872`, about 44k lines including
  evidence) predates the Neon TUI and conflicts with it throughout `src/tui`; it is
  kept as a reference implementation for the deferred B orders, not cherry-picked.

### Audit items (B) and harvest (H)

| Order | Audit finding | Deliverable | Overlap with N01–N09 / 0.7 | Plan in this cycle |
| --- | --- | --- | --- | --- |
| B01 | audit A03 | Editing a connection keeps its context, steps, inference settings and uncensored claim (typed patch, not a rebuilt profile) | none | build |
| B02 | audit A04 (part) | Text typed while the first connection starts is never overwritten by the submitted prompt | none | build |
| B03 | audit A13 | Authenticated Ollama works end to end (the relay already sends the bearer key) | A03 hosted providers | build with A03 |
| B04 | audit A14 | Headless `extensions check` refuses to start the host program unless access is Full | A07 | build with A07 |
| B05 | audit A24 | Owned llama-server gets a per-launch API key; readiness must authenticate | A07 item 6 | build with A07 |
| B06 | audit A10 | An explicitly selected runtime or engine path never silently falls back to another binary | none | build |
| B07 | audit A18 | Manual model ID entry is not on Ctrl+M (terminals send Enter); allowance recovery reachable by keyboard | N06 palette already lists *Inference allowance* (partly done) | build the manual-entry part |
| B08 | audit A19 | A cancelled connection check never reopens the model dialog | none | build |
| B09 | audit A12 | GGUF import rejects a magic-only/truncated header | none | build |
| B10 | audit A21 | Oversized paste gives visible feedback instead of being silently dropped; 60×18 form overlap | N04 keeps the 60×18 notice (partly done) | build the paste feedback; 60×18 form deferred |
| B11 | audit A25 | Smaller package: installed historical evidence | A09 | build with A09 (package excludes evidence) |
| B12 | audit A01, A08, A09, A20, A23 | F01: verification describes the executed source; spooled receipts; env freshness | none | deferred, needs design review; reference `bb34872` |
| B13 | audit A02, A28 | F02: displaced edits and file modes preserved across apply/undo races | A06 touches edit tools only | deferred |
| B14 | audit A04 (rest) | F03: durable per-session drafts across New/quit | none | deferred |
| B15 | audit A05 | F04: evolving task contract in Compact mode | none | deferred |
| B16 | audit A06, A07, A22 | F05: retrieval not suppressed by a filename; diagnostics mapped to the right crate | A06 edit robustness is adjacent | deferred |
| B17 | audit A11, A15 | F07: one owner for runtimes during benchmark/qualification; MCP helper reaping | none | deferred |
| B18 | audit A16, A17, A27 | F11: inference-trace retention, staged install generations | none | deferred |
| B19 | audit A26 | F13: matched baseline campaign | A10 prepares the PC script | owner PC |
| B20 | — | F14: training only after qualification | none | deferred |
| B21 | audit A22 (scripts) | Instruction optimiser reads stdout as well as stderr | none | deferred with B16 |
| H01 | — | Graphical Home dashboard from `build/final-pass` (status cards and action card grid on wide terminals, list at 80×24), drawn through main's theme engine | N03/N04 own the theme; Home layout itself is not in N01–N09 | build |
| H02 | — | Custom accent color from `build/final-pass` | N03 themes | deferred (theme engine owns colors) |

Order of work: board (this section) → A07 (+B04, B05) → A01 → A02 → A03 (+B03) → A04 →
A05 → A06 → A09 (+B11) → A08 and A10 prepared for the owner PC → B01, B02, B06–B10, H01.

## Status (cloud, 2026-10-11)

Branch `cloud/0.7-2026-10-11`; fixtures only (no keys, no GPU, no weights). Every
commit ran fmt, strict Clippy, `cargo test`, the TUI smoke scripts relevant to it and
a secret scan; the final head passed the complete `scripts/setup-cloud.sh` (173 Rust
tests, 72 Python unit tests, all TUI/Goose/compact/stream-recovery smokes).
Screenshots per order are in `docs/screenshots/0.7/<order>/` at 80×24 and 120×40.

| Order | Commit | Result | Deferred |
| --- | --- | --- | --- |
| Board | a9318a4 | Work orders, trial, audit and harvest plan on main; every setting in the palette | — |
| A07 | 7bed166 | Scoped trust and `--allow-terminal`; data folder protected; case-insensitive secret names; loopback-only plain HTTP auth; runtime API key file; headless extension check needs Full (B04, B05); hashed dev requirements | `persist-credentials: false` on the three evidence jobs that `git push` |
| A01 | 3508ee8 | Thinking control per dialect; cutoff detection, labelled recovery, typed headless event | — |
| A02 | 190a4c1 | Penalties, seed, stop, typical-p, extra JSON; Ollama `num_ctx` + `/api/ps` read-back; named llama-server options; `alt params` | Whether a given Ollama version honours `num_ctx` on `/v1` is measured on the PC (Alt reports what loaded) |
| A03 | f801b52 | 13 presets, `alt init --preset`, catalog search; key never stored (smoke-proven); B03 Ollama auth | — |
| A04 | 9ba7be1 | Retries with Retry-After; output guard discards degenerate output once | — |
| A05 | 86c1ab8 | Bounded probe with labelled, opt-in suggestions | — |
| A06 | 76529ee | `replace_file`; whitespace-tolerant `old_text` with closest-span handle | Effect measured by A10 |
| A09 | f322b8f | New-user README; package docs 96% smaller (B11); weight rule in CI | — |
| A08 | 93b788c | GPU detection, pinned installer, explicit selection, pin and PC check scripts | Pin slot empty until a build is verified on the owner PC |
| A10 | eb02759 | Truthful labels; `pc_matrix_0_7.sh` validated by `--dry-run` | The run itself (owner PC) |
| B01 | 518518c | Connection edits keep custom settings | — |
| B02 | ccf9807 | Text typed while connecting is kept | Durable drafts across New/quit (B14) |
| B06 | 258374f | Selected runtime/engine never falls back | — |
| B07 | 73812c4 | Manual model ID on Alt+M and a button | — |
| B08 | 999b3f2 | Cancelled connection check opens nothing | — |
| B09 | 6acd751 | GGUF fixed-header validation | Import shard-set discovery (with B12) |
| B10 | 05ea611 | Oversized paste explained | 60×18 connection form |
| H01 | 6042ae4 | Home dashboard on wide terminals | H02 custom accent colour |
| B12–B21 | — | Not started this cycle | F01, F02, F03 rest, F04, F05, F07 rest, F11, F13, F14; reference implementation in `build/final-pass-2026-10-08` (`bb34872`) |
