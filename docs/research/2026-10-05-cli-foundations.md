# Choosing a Rust foundation for alt-cli

**October 5, 2026 — source research; runtime comparison pending.**

## Recommendation

Start with a **CLI-focused custom distribution of [Goose][goose]** as the first implementation candidate. Test it against **[Stakpak][stakpak]**, the most interesting less-obvious alternative found in this research. Keep **[AIChat][aichat]** as the lightweight control and possible fallback if a compact terminal interface becomes more important than retaining a substantial agent runtime.

Goose is the leading choice because the requested product needs an existing agent loop, tools, provider adapters, sessions, and extension management. Its Rust implementation supplies those pieces, and its maintainers explicitly document [custom distributions][goose-distros]. We can begin with configuration, a small tool pack, and narrowly scoped changes instead of designing the whole application.

This is a recommendation about the **fit of the source architecture**. It does not establish that Goose outperforms Stakpak with a 4B model. That decision needs the [same-task comparison](2026-10-05-evaluation-plan.md).

An alternative for more control is **[Rig][rig] plus a terminal shell**, but that requires building more product infrastructure. It is a good library foundation, not the shortest route to an existing working CLI.

## Scope and research method

The intended product is an interactive terminal agent that can also run noninteractively, use small local models and optional hosted models, and operate software-testing tools. OpenCode was excluded as a base, as requested. The search remained open to general agents, developer tools, DevOps agents, libraries, and inference projects.

Discovery used live GitHub Rust topic listings, followed by primary repositories. Inspection covered README files, Cargo manifests, license texts, selected provider implementations, and tool-loop/execution code. The [source manifest](sources.json) records exact revisions and retrieved file hashes. These revisions identify the evidence; a production dependency should use a tested release or deliberately pinned revision.

No candidate binaries were built, no memory claims were independently measured, and no live model benchmark was run. A project advertising a capability is weaker evidence than an inspected implementation; neither proves end-to-end reliability. GitHub stars and vendor benchmark graphics were not used to rank the candidates.

## Candidate comparison

License entries below concern the inspected project's code. Dependency licenses, bundled assets, model licenses, and trademarks need separate consideration when distributing a product.

| Candidate | What it supplies | Local-model and tool evidence | Reuse judgment |
|---|---|---|---|
| **Goose — Apache-2.0** | Rust agent core, CLI, sessions, MCP extensions, multiple interfaces | Ollama implementation, OpenAI-compatible client, LM Studio documentation, execution limits and permission inspections; optional experimental tool shim | **First choice for an existing agent base.** Broad feature surface requires a deliberate CLI build and small-model defaults. |
| **Stakpak — Apache-2.0** | Rust DevOps CLI/TUI; separate AI client, agent core, MCP, and shell-approval crates | Documented local OpenAI-compatible profile with optional API key; source contains bounded retries/turns, context reduction, tool-execution contract | **Strongest alternative.** Particularly suitable if command orchestration and auditable tool execution dominate the product. Remove assumptions about DevOps/vendor services where necessary. |
| **AIChat — MIT OR Apache-2.0** | Rust chat REPL, shell assistant, sessions, RAG, executable tools | Configurable OpenAI-compatible endpoint; tools launch external executables; MCP via the companion llm-functions bridge | **Best compact prototype/control.** More execution hardening and agent-state work would fall to us. |
| **Rig — MIT** | Rust provider abstractions, agent runtime, typed tools, MCP integration | Current agent documentation describes a serializable execution state machine, turn budgets, tool validation/recovery; separate rmcp integration | **Best library route.** We still own the terminal, process supervisor, persistence policy, packaging, and user experience. |
| **ForgeCode — Apache-2.0** | Rust coding CLI, tool configuration, agents/workflows, MCP | README documents OpenAI-compatible provider configuration and MCP | Worth keeping as a coding-focused alternative. The advertised sandbox example creates a Git worktree; that alone is not process isolation. Source inspection was narrower than for the finalists. |
| **jcode — MIT** | Rust coding TUI, extensive provider integrations and tools | README explicitly documents Ollama, LM Studio, arbitrary Chat Completions endpoints, MCP, and per-model context settings | Interesting working product, but a broad workspace and many integrations increase fork surface. Its documented generic 200k context fallback needs an explicit override for local models. |
| **ZeroClaw — MIT OR Apache-2.0** | Rust agent runtime, tools, channels, routing and security components | Local-server adapters and a custom Chat Completions route; source-level Ollama caveat below | Promising runtime/components. The broad persistent-assistant scope and current Ollama native-tool behavior make it a secondary choice. |
| **Moltis — MIT** | Persistent Rust personal-agent server, sandbox/tool infrastructure, memory and many integrations | README describes local provider support and MCP; license found in `LICENSE.md` | Useful architecture reference. Persistent server and communication integrations exceed the initial terminal scope. |
| **OpenFang — Apache-2.0 OR MIT** | Rust agent platform with runtime, CLI, memory, extensions and daemon surfaces | README names Ollama, vLLM, LM Studio and MCP; Cargo manifest confirms Rust workspace and dual license | Broad platform rather than a focused terminal base. Security/performance claims in its README were not audited. |
| **Codex CLI — Apache-2.0** | Substantial Rust coding-agent implementation and terminal tooling | Current provider source includes Ollama/LM Studio but its wire protocol enum is Responses-only | Valuable reference for terminal behavior and execution controls. **Poorer fit for arbitrary Chat Completions-only servers** without extra compatibility work. |
| **Nanocodex — MIT OR Apache-2.0** | Embeddable Rust agent lifecycle plus terminal consumers and tool crates | Its README explicitly preserves separate OpenAI Responses and Claude Messages runtimes and says there is no provider/model portability layer | Good lifecycle reference, but that declared boundary conflicts with our broad local-backend goal. |
| **mistral.rs — MIT** | Rust inference engine/server with agentic execution features | README documents compatible serving, tool calling, grammar enforcement, MCP, and agentic loop | Investigate as an optional backend. Making it the entire product would couple the CLI to one inference stack. |

Primary candidate sources: [Goose][goose], [Stakpak][stakpak], [AIChat][aichat], [Rig][rig], [ForgeCode][forge], [jcode][jcode], [ZeroClaw][zeroclaw], [Moltis][moltis], [OpenFang][openfang], [Codex][codex], [Nanocodex][nanocodex], [mistral.rs][mistralrs].

## Security-specific projects checked

The research also covered projects whose purpose is already security testing. These are useful references or potential tool integrations, even though they do not match the proposed Rust terminal foundation.

| Project | Source finding | Relevance to alt-cli |
|---|---|---|
| **[PentestGPT][pentestgpt] — MIT** | Current README describes a Python 3.12+ autonomous pipeline driven by Claude Code or Codex; its interactive/legacy mode separately lists direct providers and local Ollama | Useful workflow/evaluation reference. Do not confuse the local-provider capability of one mode with the main autonomous pipeline. It would not remove our Rust and small-model integration work. |
| **[PentAGI][pentagi] — MIT** | Documentation describes a Go backend, React/TypeScript UI, PostgreSQL/pgvector, containerized execution, provider integrations, tool-call limits and repeated-call detection | Worth studying for persistent evidence and bounded security workflows. The service stack and multi-agent design exceed the initial portable CLI target. Its isolation claims were not audited. |
| **[HexStrike AI][hexstrike] — MIT** | Python MCP security-tool integration project with a large advertised tool inventory | Potential source of narrowly selected tool integrations. Attaching the full catalog would work against the small-context design. Review each wrapper's arguments, dependencies, output, and process behavior before adopting it. |

This supports separating the **agent host** from **security tool packs**. We can reuse a Rust host while integrating a small, tested subset of external tools through MCP or typed executables. A security-branded agent is not automatically a better small-model host.

## Findings that change the decision

### Goose already has a customization path

The [distribution guide][goose-distros] covers provider configuration, bundled MCP extensions, system prompts, workflows, branding, and alternate interfaces. That is unusually close to this project's intended reuse model.

The Rust CLI and core are distinct from the desktop interface. However, “CLI only” does not automatically mean a minimal build. The inspected [CLI manifest][goose-cli] enables features including embedded inference, code mode, cloud-provider integrations, telemetry, voice, bundled MCP, and updates by default. A small distribution needs an explicit, tested feature selection. This research did not verify a reduced build command.

The inspected [agent source][goose-agent] includes permission and repetition inspections and configurable turn limits. Its `DEFAULT_MAX_TURNS` is **1000** at the captured revision. That is inappropriate as an initial budget for a small-model security assistant; our proposed profile begins around 12 model turns for a bounded task and exposes a user-controlled continuation.

Goose also has an [experimental tool shim][goose-shim]. It can interpret tool intentions emitted as text and convert them to structured calls, using an interpreter model. This is relevant to derivatives that intermittently lose native tool formatting. It is **not** evidence that every model can reason about tools reliably. The default interpreter is `mistral-nemo`; silently adding another model would undermine the low-resource goal. Start with native tools, and evaluate a shim separately with its model and memory cost made explicit.

### Stakpak has a useful boundary around the agent runtime

The [agent-core README][stakpak-core] describes a transport-independent loop with host-provided `ToolExecutor`, hooks, compaction, and versioned checkpoints. Its [configuration types][stakpak-types] expose `max_turns`, output limits, retries, compaction, and tool policy. Source inspection also found context handling that removes orphaned tool results and trims older content.

This is attractive if we want to reuse a working product while retaining a clear place to insert typed security-testing tools and evidence logging. It also gives a plausible component-reuse path if the complete CLI is too opinionated.

The README's `profiles.offline` example points at a local OpenAI-compatible endpoint with an optional API key. Its top-level `provider = "local"` means the direct-provider path; the endpoint still determines whether inference actually stays local. Any cloud-backed search, documentation, telemetry, and update functions must be checked separately before describing a configuration as fully offline.

There is no small-model success measurement in this research. Deterministic orchestration can reduce avoidable failures, but it cannot supply reasoning the model lacks.

### AIChat is a useful starting shell, but its runner is simpler

The inspected [OpenAI-compatible client][aichat-provider] accepts a configurable base URL and optional key and calls `/chat/completions`. This is a good general local-server contract.

The [tool implementation][aichat-functions] checks registered tool names, parses arguments as JSON, launches an external executable, and reads its result through `LLM_OUTPUT`. It does not perform full JSON Schema validation at that dispatch boundary. The inspected path also does not implement the process timeouts, output quotas, target restrictions, and checkpointed execution policy proposed for alt-cli. Those controls would need an external tool runner or new implementation.

The companion [llm-functions][llm-functions] repository supports Bash, JavaScript, and Python tools and an MCP bridge. Therefore, a Rust host does not imply all tool packs are dependency-free Rust. Our own tools could be Rust executables, but the companion runtime dependencies must be accounted for.

AIChat's simple REPL, sessions, input handling, and provider configuration remain useful reference material. A message mentioning infinite-loop detection in its source should not be treated as proof of a robust cross-turn loop budget: the inspected deduplication concerns tool-call IDs in the supplied call list.

### Provider support has protocol-level exceptions

At the captured ZeroClaw revision, [the Ollama adapter][zeroclaw-ollama] returns `native_tool_calling: false` and `supports_native_tools() == false`. That does **not** establish that ZeroClaw cannot use tools: other adapters and text-based strategies exist. It does mean a general “supports Ollama” claim is insufficient for selecting the tool-calling route we want.

At the captured Codex revision, [the provider definition][codex-provider] contains only `WireApi::Responses` and explicitly removes `ollama-chat`. An endpoint implementing `/v1/chat/completions` alone is consequently not interchangeable with the local endpoints Codex expects. Newer Ollama versions can expose Responses, but the product should not make that its only portability contract.

Nanocodex's [design boundaries][nanocodex] explicitly exclude a provider/model portability layer. That is a sensible choice for that project, but additional work for ours.

### Rig is the strongest reusable-library alternative

At the inspected revision, Rig has separated `rig-core` provider/tool contracts from the `rig-agent` runtime, with a root facade. The [agent runtime documentation][rig-agent] describes a serializable state machine that handles turn budgets, validation/recovery, history, and output policy. The [MCP integration][rig-mcp] uses the Rust SDK and can expose dynamic tools.

This makes Rig more substantial than a thin API wrapper. It still does not remove the need to build a terminal product and a dependable command executor. Its current HEAD also requires Rust 1.95.0; older examples using a different crate layout need version matching. Prefer a released, tested API if choosing this route.

## Maintenance and dependency observations

For locally inspected shallow checkouts, the observed HEAD commit dates were February 23, 2026 for AIChat, July 6, 2026 for Stakpak, and October 5, 2026 for Rig and ZeroClaw. These are observations about those revisions, **not** a bus-factor assessment or proof that a project is abandoned or stable.

The captured manifests also show recent toolchains: Goose declares Rust 1.94.1, ForgeCode 1.94, and mistral.rs 1.94. Some README examples lag the manifest or current crate organization. Before implementation, pin a release, inspect its build instructions and CI, and build exactly the features we intend to distribute.

A Rust CLI's process footprint and its model's inference footprint are separate. A tiny executable does not make a 20B model small, and a large repository does not prove a large idle-memory requirement. Measure the shipped binary, tool subprocesses, and inference server separately.

## How to reuse the chosen project

For the first Goose experiment, use upstream configuration and extensions to create a recognizable alt-cli workflow: local model profiles, a small software/security-testing tool pack, bounded turns, and structured evidence. Keep a record of exactly which defaults changed. Avoid accumulating a large branding-only fork before the tool/model combination works.

If deeper changes are needed, create a normal downstream fork with explicit upstream provenance and narrowly separated product changes. Preserve required copyright, license, and notice files. Copying selected files still carries their license obligations and creates a responsibility to track upstream fixes.

For a Stakpak experiment, first exercise the documented direct-provider path and existing CLI. If component reuse proves better, evaluate `stakpak-agent-core` and its AI/provider types together; the agent-core crate's abstraction boundary does not imply it is dependency-independent.

For a Rig route, expect to implement the terminal interface, process supervision, local configuration, audit artifacts, and project workflows. Rust building blocks such as Tokio, Serde, Clap, Ratatui/Reedline, and `rmcp` are implementation options, not evidence that this route is already a complete agent.

## License implications

The repository currently carries Apache-2.0. Goose, Stakpak, ForgeCode, and Codex have Apache-2.0 code licenses in the inspected sources; AIChat and the other dual-licensed projects allow the listed license choice. MIT and Apache reuse still require preserving the applicable notices. A fork should maintain a third-party provenance record, and branding/trademark rights should not be inferred from the source license.

[Heretic][heretic] is **AGPL-3.0-or-later** software. Treat it as a separate model-preparation tool unless we deliberately choose to incorporate AGPL code and satisfy its terms. Calling a model that somebody produced with Heretic does not, by itself, mean our CLI embeds Heretic. The base weights' license and the derivative's actual terms still need inspection; a code license is not a substitute for the model card.

The Rust MCP SDK's [license text][rmcp-license] describes a transition from MIT to Apache-2.0, retaining MIT for contributions without relicensing consent and CC-BY-4.0 for documentation excluding specifications. Check the exact version and artifacts rather than relying on a badge. Model, dataset, and tool-program licenses are separate from the CLI's license.

## Decision to take forward

1. **First candidate:** Goose custom CLI distribution, external inference, native tool calling, a deliberately small tool inventory.
2. **Challenger:** Stakpak, tested on the same local endpoint, model, prompt budget, tools, and tasks.
3. **Lightweight control:** AIChat, useful for measuring how much of the result comes from orchestration versus the underlying model.
4. **Fallback architecture:** Rig when library ownership is worth the additional product work.

Choose after measuring task completion, valid tool arguments, recovery, latency, and memory. If Goose's required footprint or customization burden proves excessive and Stakpak completes the same tasks with less adaptation, switch the base. The evaluation should be capable of overturning this recommendation.

[goose]: https://github.com/aaif-goose/goose/tree/7debb275d36559ee4e81799362e7f638275dfed3
[goose-distros]: https://github.com/aaif-goose/goose/blob/7debb275d36559ee4e81799362e7f638275dfed3/CUSTOM_DISTROS.md
[goose-cli]: https://github.com/aaif-goose/goose/blob/7debb275d36559ee4e81799362e7f638275dfed3/crates/goose-cli/Cargo.toml
[goose-agent]: https://github.com/aaif-goose/goose/blob/7debb275d36559ee4e81799362e7f638275dfed3/crates/goose/src/agents/agent.rs
[goose-shim]: https://github.com/aaif-goose/goose/blob/7debb275d36559ee4e81799362e7f638275dfed3/documentation/docs/guides/tool-shim.md
[stakpak]: https://github.com/stakpak/agent/tree/760cd2b5984d29c2d513bb15ca33e995fae45f17
[stakpak-core]: https://github.com/stakpak/agent/blob/760cd2b5984d29c2d513bb15ca33e995fae45f17/libs/agent-core/README.md
[stakpak-types]: https://github.com/stakpak/agent/blob/760cd2b5984d29c2d513bb15ca33e995fae45f17/libs/agent-core/src/types.rs
[aichat]: https://github.com/sigoden/aichat/tree/82976d349ad97ac9aae0655ad631dace5e2a6385
[aichat-provider]: https://github.com/sigoden/aichat/blob/82976d349ad97ac9aae0655ad631dace5e2a6385/src/client/openai_compatible.rs
[aichat-functions]: https://github.com/sigoden/aichat/blob/82976d349ad97ac9aae0655ad631dace5e2a6385/src/function.rs
[llm-functions]: https://github.com/sigoden/llm-functions/tree/616d8d84c5565fdb66d8782ae5ba56d994bfa82a
[rig]: https://github.com/0xPlaygrounds/rig/tree/936da797dc3b63c18768846fb120500aae7a5c12
[rig-agent]: https://github.com/0xPlaygrounds/rig/blob/936da797dc3b63c18768846fb120500aae7a5c12/crates/rig-agent/README.md
[rig-mcp]: https://github.com/0xPlaygrounds/rig/blob/936da797dc3b63c18768846fb120500aae7a5c12/crates/rig-rmcp/README.md
[forge]: https://github.com/tailcallhq/forgecode/tree/5f0fef4124e6965a059bbf4011e434db7df9720f
[jcode]: https://github.com/1jehuang/jcode/tree/6144a6f03f5e6ca26efb1aeca8a3e1ef69ee9f8a
[zeroclaw]: https://github.com/zeroclaw-labs/zeroclaw/tree/3c8a6763621ac392e471f35be0d9ebc037fefa29
[zeroclaw-ollama]: https://github.com/zeroclaw-labs/zeroclaw/blob/3c8a6763621ac392e471f35be0d9ebc037fefa29/crates/zeroclaw-providers/src/ollama.rs
[moltis]: https://github.com/moltis-org/moltis/tree/1f6d28ea750d6654d52d5899b8be67727ebf7a19
[openfang]: https://github.com/RightNow-AI/openfang/tree/acf2587e46be174c10200489c9a2d23a39a98aeb
[codex]: https://github.com/openai/codex/tree/a6c5f3a9fa8e7ccfba7d53921d2f06d06d3b9f79
[codex-provider]: https://github.com/openai/codex/blob/a6c5f3a9fa8e7ccfba7d53921d2f06d06d3b9f79/codex-rs/model-provider-info/src/lib.rs
[nanocodex]: https://github.com/gakonst/nanocodex/tree/33989caf4add7f903c87d05095ed38128a2aecbc
[mistralrs]: https://github.com/EricLBuehler/mistral.rs/tree/3f2515e9b5adc2ac44949c128c50a13b15721294
[heretic]: https://github.com/p-e-w/heretic/tree/208c0ca35b3feade91dc74755cb80b529affa325
[rmcp-license]: https://github.com/modelcontextprotocol/rust-sdk/blob/79437f291b2c44053d00dcd5db969fd0cca7c887/LICENSE
[pentestgpt]: https://github.com/GreyDGL/PentestGPT/blob/e8b1bb77d1ac00329675cec3b060aba971ec1ac8/README.md
[pentagi]: https://github.com/vxcontrol/pentagi/blob/b611fbebcf55b6deb88ad1fb7bf1f1ce7a357242/README.md
[hexstrike]: https://github.com/0x4m4/hexstrike-ai/blob/d689933ff579d839c676c82b231f8e98326c5f04/README.md
