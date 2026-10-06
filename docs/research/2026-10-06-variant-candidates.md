# Exact Spark-X2.5 and MiMo derivative discovery

October 6, 2026. Public Hub metadata and pinned cards, refreshed during 0.4 validation. No inference or weight download was performed for these candidates. These are publisher-labelled uncensored/abliterated artifacts, not independently established behavior or coding quality.

| Candidate | Pinned revision | Q4_K_M bytes | Publisher license |
|---|---|---:|---|
| [darioooooo0o/Spark-X2.5-1.7B-Abliterated-GGUF](https://huggingface.co/darioooooo0o/Spark-X2.5-1.7B-Abliterated-GGUF/tree/0193e8f9b55c7f8e9c58799694696a17d8f6c6c5) | `0193e8f9b55c7f8e9c58799694696a17d8f6c6c5` | 1258452320 | apache-2.0 |
| [ItsOkayNow/Spark-X2.5-4B-Heretic-GGUF](https://huggingface.co/ItsOkayNow/Spark-X2.5-4B-Heretic-GGUF/tree/46aa0f5327a5e3f0ffa17aa2074dd2a382e8bd67) | `46aa0f5327a5e3f0ffa17aa2074dd2a382e8bd67` | 2600225184 | mit |
| [mradermacher/MiMo-V2.6-Distill-Qwen-9B-heretic-GGUF](https://huggingface.co/mradermacher/MiMo-V2.6-Distill-Qwen-9B-heretic-GGUF/tree/4e642ce6d23ee0130bdf57cfe64d644008c042ee) | `4e642ce6d23ee0130bdf57cfe64d644008c042ee` | 5629106560 | mit |

The 1.7B Spark is the most relevant next small-machine trial. Hub metadata records `spark2_5` and a claimed 1,048,576-token native limit. That limit is not a practical RAM/VRAM budget or evidence of useful recall. The card requests an architecture-compatible runtime and f16 KV cache. The pinned llama.cpp b11429 source registers `spark2_5`, but registration is not a successful load or tool-call test.

The MiMo candidate is the 9B distillation into a Qwen3.5 architecture, distinct from the much larger MiMo MoE family. Its Q4 weight file alone is 5.63 GB; assess KV cache, runtime memory and offload on the actual 8 GB card. Keep the profile opt-in until measured. Its pinned metadata names `qwen35`, also registered in the pinned runtime.

The 4B Spark community card declares MIT while the 1.7B/base-family material declares Apache-2.0. Check the full upstream/derivative licensing chain before redistributing weights; a Hub metadata field alone is not a distribution review. Alt does not bundle these artifacts.

Use Models → Hub search/open repository to acquire a chosen candidate, select a compatible runtime, start at 4K, and run native-tool and independent project acceptance. Require SHA256/revision integrity, successful load/template/tool results, repeated task oracles, truthful prose, memory/latency, and physical-GPU fit before labelling it recommended. A failed attempt stays in the evaluation record.

Raw discovery: [search results](2026-10-06-variant-search.json), [pinned metadata, cards and checksums](2026-10-06-variant-candidates.json). Search terms can miss other derivatives; this is not an exhaustive inventory.

## Subsequent 0.4 checks

The 1.7B Spark and 9B MiMo Q4_K_M files were subsequently downloaded, SHA256-verified and exercised through the managed runtime. See [the live evaluation](../LIVE_EVALUATION.md) for successful tool probes, actual source outcomes, timeouts and resource limits. The 4B Spark listed above remains metadata-only.
