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
