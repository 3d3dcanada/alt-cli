# 0.9 work orders: usable for small projects on a hosted and a local model

Owner direction (2026-11-19): make Alt functional for real small projects with
both a hosted model (NVIDIA NIM, `NVIDIA_API_KEY`) as the daily driver and a
local 9B GGUF on the reference GTX 1070 8 GB (offline). This round runs on the
owner PC against real models; the 0.7 round ran in the cloud with fixtures only,
so its fixes (A01–A06) were never measured on a real model.

Rules for every order: one order at a time, one commit per order, on branch
`pc/0.9-2026-11-19`. Model prose is never the verification authority; a task
passes only when an independent check passes. Failures are retained. Models
run with Guided access only. Raw evidence stays off git; summaries go under
`docs/research/`. No tag, no release from this branch.

| Order | Deliverable | Depends |
| --- | --- | --- |
| P01 | Checkout, pinned toolchain, baseline gates, version `0.9.0-dev` | |
| P02 | CI green (dependency advisories) and stale README model IDs | P01 |
| P03 | Hosted turns: where the seconds go, waiting lines, clean timeout, probe retries | P01 |
| P04 | CUDA 12 runtime for Pascal built and run on the reference GPU | P01 |
| P05 | Measured matrix on real local and hosted models | P03, P04 |
| P06 | Fix the top two failure classes from P05 (capped) | P05 |
| P07 | Small-project acceptance on the best hosted and best local model | P05, P06 |
| P08 | Docs from real results, pull request, owner-PC install | P01–P07 |

## Baseline trial (owner PC, 2026-11-19, installed 0.6.0 build)

Task: a two-file Python project with a failing test (two one-character bugs).
`alt run '<fix the tests>' --allow-tools --json`, scratch data folder, Guided access.

| Model on NVIDIA NIM | Wall time | Outcome |
| --- | --- | --- |
| moonshotai/kimi-k3.1-pro, run 1 | 406 s (hit `--timeout 400`) | read both files, then silent; no edit |
| moonshotai/kimi-k3.1-pro, run 2 | 181 s (hit `--timeout 180`) | correct edit applied, then turn cancelled by the timeout |
| z-ai/glm-5.3-flash | 246 s (hit `--timeout 240`) | no edit; "Engine did not stop within 5 seconds" |
| minimaxai/minimax-m3.1-flash | 214 s | correct edit, clean `end_turn`, exit 0 |

Other observations: a direct one-line chat completion to the same provider
answered in about 10 s while single engine requests took 40–140 s; `alt probe`
failed on one HTTP 529 without retrying; nothing is printed in headless text
mode while a request is in flight; `z-ai/glm-5.3` (README example) is end of
life (HTTP 410).

## Status

| Order | Result | Open |
| --- | --- | --- |
| P01 | Pinned 1.99.0; fmt, strict Clippy, 200 tests and release build green at normal load; version `0.9.0-dev`; baseline re-run: 697 s, edit correct (independent check pass), exit 1 after `--timeout 600` overran by about 97 s | Wall-clock-bound TUI lock tests (`recovery_tests`) flake under heavy host load, on unchanged `main` too |
| P02 | `cargo audit --deny warnings` clean: rustls 0.23.46, rustls-webpki 0.103.17, tokio 1.53.3, rustix 1.1.6, rusqlite 0.41.1 (`libsqlite3-sys` 0.39.1) with its `fallible_uint` feature so integer columns keep checked conversions without call-site changes; an existing sessions database lists and exports identically with the old and new build; README model ID and status updated | — |
| P03 | 18 timed runs of the baseline task on three hosted models. Session store moved to `synchronous=NORMAL` (WAL): each streamed chunk is one append and a full fsync averaged 550 ms (worst 5.6 s) on the reference disk, so replies lagged up to a minute behind the model. Retry backoff base 0.5 s to 2 s. Headless waiting line every 15 s (`waiting` event); `--timeout` exits 124 and lists changed files (`timeout` event); `alt probe` retries like the relay. Alt+engine time per task 0.6–8.9 s at normal load (was up to 290 s); passes 5/9 (was 4/9) | Remaining misses are provider-side (one model needs about 300 s per request; 429/529 bursts). Under heavy disk contention each tool call still waits on the project journal's full fsyncs (journal durability kept on purpose) |
| P04 | `scripts/build-pascal-runtime.sh` built llama.cpp (pinned commit) with CUDA 12.5 for sm_61 in 636 s; `--list-devices` shows the GTX 1070. Script accepts `CMAKE=` (a broken pip `cmake` shim on PATH stopped the first attempt). `pc_cuda_check.sh`: MiMo 9B Q4_K_M fully offloaded, q8 KV, flash attention, 8K: prompt 157 tok/s, generation 28 tok/s, peak 5,567 MiB. Selected through the TUI (GPU runtime → Select an existing llama-server; context 16K): baseline task in 111.7 s, exit 0, independent check pass, peak 5,750 MiB at 16K, VRAM back to idle after | Pin constant stays empty: publishing the runtime archive is an owner decision |
| P05 | Matrix script for 0.9 (Guided, GPU offload, serial, resumable); pass 1: 112 runs. Best local Spark 4B 5–6/8, best hosted Kimi 7/8. Summary in `docs/research/2026-11-pc-matrix-0.9/` | Every Guided check was refused on the reference PC (bwrap blocked by AppArmor). Pass 2 stopped part way. Top classes: wrong-edit 30, timeout 18 |
