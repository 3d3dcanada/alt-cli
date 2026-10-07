# alt-cli research

**Reasoning follow-up, October 7, 2026:** [Strengthening reasoning in small local
models](2026-10-07-small-model-reasoning.md), with
[eight proposed reasoning work orders](2026-10-07-reasoning-experiments.md) and a
[source ledger](2026-10-07-reasoning-sources.json). It covers bounded thinking,
verified candidate search, decomposition, small-model distillation/RL and
abliterated reasoning-model candidates. These are researched proposals, with
published claims distinguished from Alt's measurements.

**Latest research, October 7, 2026:** [Making small local models more capable in
Alt](2026-10-07-small-model-harness.md), with an
[experiment/work-order proposal](2026-10-07-small-model-experiments.md),
[pinned source ledger](2026-10-07-small-model-sources.json) and
[Alt failure diagnostics](2026-10-07-alt-failure-diagnostics.json).
It prioritizes simpler edits, executable skills, host-managed workflows and
model-specific budgets. These are researched proposals; no new model gains are claimed.

## Original foundation research

Research date: **October 5, 2026**. Scope: a Rust terminal agent for small local models, including abliterated/uncensored derivatives, with tools for authorized software and security testing.

**Recommendation:** evaluate a CLI-focused custom distribution of **Goose** first, with **Stakpak** as the strongest alternative. Use AIChat as the lightweight comparison. Keep inference in a separate server initially. Select the final base after the same small-model tool tasks run on both finalists.

**Latest product plan:** [Build plan](../BUILD_PLAN.md). It specifies our own Rust TUI over a pinned Goose engine, model downloads, older-computer support, tool packs and context management. The confirmed reference machine is Linux with a GTX 1070 (8 GB VRAM) and 16 GB RAM. All live prototype tests must use uncensored/abliterated models. Spark-X2.5 is now identified and included in the shortlist; its uncensored derivative remains to be verified.

- [CLI foundations and recommendation](2026-10-05-cli-foundations.md): broad candidate comparison, source findings, licensing, and reuse strategy.
- [Models, serving backends, and small-model design](2026-10-05-models-and-backends.md): Heretic compatibility, model shortlist, backend contracts, and proposed architecture.
- [Evaluation and decision plan](2026-10-05-evaluation-plan.md): concrete experiments and acceptance criteria before a fork becomes the product.
- [Source manifest](sources.json): immutable repository revisions, retrieved paths, timestamps, and content hashes.

The initial research is separate from the subsequent [implementation and live evaluation](../IMPLEMENTATION.md). Repository documentation, manifests, licenses, and selected implementations were inspected. Subsequent small live CPU checks are recorded in [LIVE_EVALUATION.md](../LIVE_EVALUATION.md); no GTX 1070 or broad task-suite benchmark has run.

Hugging Face's own GitHub projects and model authors' repositories informed the research. The requested Hugging Face plugin did not expose callable tools. Direct Hub access initially returned a proxy 403; metadata/cards became accessible during implementation. A specific [Heretic GGUF candidate](live-model-candidate.json) is now pinned, with its parent/quantizer cards in the source manifest. Other candidate cards remain pending verification. CDN access subsequently became available. The exact GGUF was downloaded through Alt and its full SHA256 verified. Live CPU checks include both successes and an independently detected model failure. Saving the reusable environment draft still does not publish it.

## 0.4 refresh

[Exact Spark-X2.5 and MiMo derivative candidates](2026-10-06-variant-candidates.md) includes pinned cards/checksums and distinguishes discovery from successful inference.
