# 2026-10-08 PC model trial

Owner PC: Linux, GTX 1070 8 GB, 16 GB RAM. Alt `v0.6.0-beta.3` built from `main`
(`cargo build --locked --release`), managed Goose 1.53.0. All 123+ Rust tests and
strict Clippy passed before the trial.

Harness: `scripts/live_acceptance.py --suite v5 --repeats 1 --contexts 16384
--tool-profile compact --timeout 600`, 8 cases: python-feature, python-multifile,
rust-feature, javascript-config, python-http-security, heldout-toposort,
sealed-intervals, sealed-cache-expiry. Pass = independent oracle passes.
One repeat each, so treat differences of one task as noise.

| Model | Where | Output | Pass | Failure classes |
| --- | --- | --- | --- | --- |
| Spark-X2.5 4B Q8 (abliterated) | Ollama, GPU | 1,024 | 0/8 | 7 budget (all tokens spent in `reasoning`, zero tool calls), 1 wrong |
| Spark-X2.5 4B Q8 (abliterated) | Ollama, GPU | 4,096 | 3/8 | 2 budget, 3 wrong |
| MiMo V2.6 Distill 9B Q4_K_M abliterated | llama.cpp CUDA 12, full offload, q8 KV, ~6.3 GB VRAM | 4,096 | 3/8 | 4 wrong, 1 timeout |
| MiMo V2.6 Distill 9B Q4_K_M standard | same | 4,096 | not measured | session ended before the run |
| `moonshotai/kimi-k3` | NVIDIA NIM | 4,096 | 4/8 | 2 provider garbage, 2 wrong |
| `z-ai/glm-5.3` | NVIDIA NIM | 4,096 | 3/8 | 3 budget, 1 no tool calls, 1 wrong |

Findings that drive `docs/WORK_ORDERS_0.7.md`:

1. Reasoning control does not reach Ollama or hosted servers. The relay sends only
   llama.cpp's `reasoning_budget_tokens`. Thinking models spend the output
   allowance before acting (Spark 1,024: 8/8 runs had only `reasoning` deltas).
   Output budget exhaustion is the largest single Alt-side failure class.
2. `inference.rs` removes `options`, so the profile context never reaches Ollama;
   the trial needed a hand-made tag (`PARAMETER num_ctx 16384`).
3. NVIDIA Kimi K3 intermittently streams multilingual garbage with leaked template
   tokens (`<|open|>`, `<|close|>`) after a tool result. Replaying the same request:
   about 1 in 5 garbage at default temperature and at 0.6, plus HTTP 429. Provider
   fault, but Alt has no detection or retry.
4. rust-feature failed for every model: the oracle requires
   `median(&[i64::MIN, i64::MAX]) == Some(-0.5)`, which needs i128 arithmetic.
   f64 averaging gives 0.0. A fair hard case, not a harness bug.
5. Hosted runs needed `--api-key-env` in the harness (added on this branch). The
   harness still forces `--uncensored`, which mislabels standard and hosted runs.

Raw run directories are on the owner SSD under `3D3D-Launch-2026-10-08/Software/Alt-CLI/alt-cli-trials/` (not committed,
they hold full request/response logs).
