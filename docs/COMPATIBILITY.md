# Compatibility and qualification

Alt 0.6.0 beta 3 is a Linux x86_64 supervised beta. A compatible endpoint can
connect successfully while its selected model still makes incorrect tool calls
or changes. Check the exact artifact, template, runtime, context and task together.
The [PC recorder](PC_TESTING.md) performs that check using your own selection.

## Application and adapters

| Component | Evidence | Practical limit |
| --- | --- | --- |
| OpenAI-compatible adapter | Actual Goose with streaming, native edits/checks, twelve-turn continuation, external-edit recovery and dirty Git files | API shape alone does not establish native tool quality for every server/template |
| Ollama adapter | Same weight-free protocol journeys through Ollama-shaped endpoints | Actual Ollama model, GPU allocation and speed require local qualification |
| Owned llama.cpp | Pinned b11429 CPU runtime; actual GGUF trials, rendered chat/tool counts, recorded usage and lifecycle | GPU kernels, offload and available memory depend on your runtime build |
| LM Studio / ORA / other compatible endpoints | Configurable endpoint and exact selected model; no automatic substitution | Individual product/version combinations have not all been exercised here; external tokenization/lifecycle can remain unknown |
| Linux frontend | Portable package installation, TUI, integrity and upgrade/rollback on Debian 11 and Ubuntu 24.04 | Frontend compatibility does not imply the separately installed runtime has the same libc minimum |
| Narrow terminal | Actual 80×24 allowance/reconnect journey and existing smaller-screen journeys | Human beginner usability remains a separate acceptance session |

Protocol fixtures exercise actual Alt/Goose behavior without weights. They
establish adapter/application behavior, not model intelligence. The package's
`PLATFORM.json` records exact libc requirements; managed Goose requires glibc
2.28+, and the managed CPU llama.cpp build requires glibc 2.34+. Use an external
compatible runtime on older distributions where necessary.

## Exact model evidence

| Artifact / cohort | Observed result | What it establishes |
| --- | --- | --- |
| MiMo-V2.6-Distill-Qwen-9B-heretic Q4_K_M, B05 matched CPU pilot | Original and compact tools each passed 3/4 tasks at 8K; compact used less input | A small matched pilot, not a general improvement rate |
| Same MiMo artifact, B06 | 6/9 independently checked attempts, including 2/4 held-out tasks and an actual 80×24 TUI repair | Some bounded Python repairs work; two Rust feature attempts still failed |
| Same MiMo artifact, B07 continuation | A two-turn TUI trial passed with a 24-request connection cap; a prior 12-request trial failed | A retained continuation success; cap changes are not a controlled causal comparison |
| Josiefied-Qwen2.5-7B-Instruct-abliterated Q4_K_M, earlier repair cohort | 0/2 scalar repairs; another line-array attempt failed | Native compatibility does not qualify reliable 7B repair behavior |
| Spark-X2.5 abliterated / Qwen3-4B Heretic, earlier campaigns | Saved incomplete repairs and native/coding failures | No promoted coding preset; earlier frozen failures remain unchanged |
| MiMo 9B, beta 2 development practice | Repair, follow-up and cancellation-resume each passed four independent cases; persisted goal and undo passed | A complete bounded CPU practice journey using the selected artifact |
| Josiefied 7B, beta 2 development practice | Same complete journey passed, including recovery after incorrect intermediate edits | A task-specific 7B success, separate from earlier failed repair families |
| MiMo 9B, first beta 2 development Rust feature | Toolchain environment prevented original verification; a separate configured check of retained source failed an extreme-value case | Infrastructure failure and actual incorrect source are recorded separately |
| MiMo 9B, configured Rust requalification | Language-tool preflight and failing seed executed correctly; final independent acceptance failed and the turn reached the CPU deadline after twelve calls | This Rust task remains unqualified; no quality preset is promoted |

Beta 2's fresh development trials and complete raw outcomes are recorded in
[the PC completion evidence](evidence/pc-ready/README.md). They use 8K context,
1,024 output tokens and a finite 12-request / 8,192-generated-token allowance per
connection, with ten-minute CPU turn deadlines. CLI resume starts a new
connection; these limits do not cover the whole multi-turn recorder as one pool.
Every failure remains in the record. Model trials do not use fixture responses.

Pinned GGUF digests:

- MiMo 9B: `00877ff79174b79f72c5ec704901fc958400a0b0d390fb61a068281618116dd1`.
- Josiefied 7B: `d7d626d96cc2d3567e8266101b4211b17634012dd99a0c9b28f475ce4eb620b6`.

## Your 16 GB RAM / GTX 1070 computer

Start with 4K or 8K context and one loaded 7B/9B Q4 model. Weight size excludes
KV cache, runtime buffers and other processes. Measure available RAM, VRAM,
actual GPU layers and latency before increasing context. The GTX 1070 uses
Pascal sm_61; choose a compatible CUDA 12 runtime or CPU mode. A CUDA 13-only
runtime does not cover that card. The managed default is CPU inference.

This cloud has no NVIDIA GPU. Its CPU results do not establish GTX 1070 fit,
throughput or a 16 GB desktop's usable memory. Arbitrary model names, uncensored
labels and long advertised context do not establish native-tool reliability.
Full access supports ordinary host commands, network and selected external
tools; inference/template limitations still apply.

No model weights were trained in this completion pass. The existing
[training handoff](../training/README.md) requires a rights-reviewed,
independently checked corpus and supported full-precision model loader before
training. Q4 GGUF inference artifacts are not the training inputs.
