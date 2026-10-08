# Owned real-model lifecycle receipt (2026-10-08)

This is a bounded CPU lifecycle check, not a task-quality campaign or GPU compatibility result. It uses the explicitly abliterated Josiefied-Qwen2.5-7B checkpoint, SHA-256 `d7d626d96cc2d3567e8266101b4211b17634012dd99a0c9b28f475ce4eb620b6`, existing local weights, and pinned llama.cpp b11429. No weights or binary need to be copied with these receipts.

`smoke.rs` is the complete standalone harness. Adjust only the local state/model/runtime paths for another machine. It imports by reference and verifies the exact digest, starts Alt's actual `LocalRuntime`, uses a 4K context/CPU 2 threads/batch 128, verifies bare proxy access returns 403, checks warm file identity, requests at most 24 output tokens, then explicitly stops and verifies the owned leader is reaped. Response content and usage are retained in `receipt.json`. The inherited HTTP proxy environment deliberately points at a closed local port; the owned client must bypass it.

In this cloud checkout, the harness was linked against the built `alt-cli` library using the `rustc` arguments recorded in `build-command.json`. Cargo artifacts have configuration-specific suffixes; after `cargo build --locked --lib`, resolve the current `libalt_cli-*.rlib` and its matching dependency artifacts from Cargo fingerprints instead of assuming those suffixes are stable. The Rust source can also be run as a temporary example with the same package dependencies.

Executed command after compilation:

```bash
HTTP_PROXY=http://127.0.0.1:1 HTTPS_PROXY=http://127.0.0.1:1 \
ALL_PROXY=http://127.0.0.1:1 http_proxy=http://127.0.0.1:1 \
https_proxy=http://127.0.0.1:1 all_proxy=http://127.0.0.1:1 \
NO_PROXY='' no_proxy='' timeout --kill-after=5s 300s \
/tmp/alt-runtime-live-smoke/smoke > /tmp/alt-runtime-live-smoke/output.log 2>&1
```

Exit status was 0. `receipt.json` records READY (2 completion tokens), load/integrity 10.456 s, generation 2.503 s, 403 without the URL capability, and successful child reaping. Runtime did not emit recognized GPU offload counts, so the observed count is null, even though 0 GPU layers were requested. `runtime.log` is the actual runtime log. Measured token speed from this tiny completion is not a representative performance benchmark.

Do not retain the `smoke` executable or `state/` directory in repository evidence. Retain only this README, source, JSON receipt/build command, and logs if needed. Local filesystem paths identify this run; they are not installation defaults or credentials.
