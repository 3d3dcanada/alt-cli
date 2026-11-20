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
