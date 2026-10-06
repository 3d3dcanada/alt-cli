# alt-cli build plan

Updated October 6, 2026. This describes the full product roadmap. The 0.4 beta implements the subsequently approved work orders across workspace, tools, context, recovery and runtime control; see [implementation status](IMPLEMENTATION.md) for the tested scope and remaining work. **All live prototype/model tests must use an uncensored or abliterated checkpoint**. Earlier suggestions to use an unmodified instruction model as the initial live baseline are superseded.

**Reference machine:** Linux, 16 GB system RAM, NVIDIA GTX 1070 with 8 GB VRAM. Prioritize older consumer computers, retain CPU operation, and enable additional acceleration/capacity on newer systems. ORA is the user's own runtime and does not require a dedicated integration in this phase.

## Approved 0.3 additions

Implemented: Alt-owned tools and execution profiles; checkpoints/diff/undo;
Task progress and direct check reruns; durable memory/current-file retrieval;
hardware guidance/native tool probe; older Linux packaging and isolated checks.
**Full access** supports arbitrary terminal commands, networking, installs and
external tools. Guided/review modes remain optional choices. Exact behavior,
verification and remaining platform/model limits are in
[CONTROLLED_WORKFLOWS.md](CONTROLLED_WORKFLOWS.md) and
[IMPLEMENTATION.md](IMPLEMENTATION.md). The detailed research roadmap below is
retained; proposed future items are not all release claims.

## Product and foundation

Build a **Rust terminal application with our own TUI, command-line mode, model manager, and software/security-testing workflows**, using a pinned Goose engine as the starting agent runtime. Keep Stakpak as the fallback if the initial integration cannot meet the required controls without excessive upstream changes.

Goose's [custom-distribution guide][goose-distros] explicitly covers rebranding, providers, extensions, prompts, and new interfaces. Apache-2.0 permits a downstream product subject to the required notices and modification attribution. Ship our name and visual identity while retaining upstream license/copyright notices and avoiding an implication of upstream endorsement. Track every reused component's origin and version.

Use a **local ACP connection over stdio** between the new TUI and the engine initially. The documented Goose interface includes streamed updates, permission requests, tool events, cancellation, sessions, and MCP configuration. Keep an internal `Engine` interface so the UI is not coupled to Goose's internal structs.

The first integration spike must verify these capabilities at the pinned version. It must also verify configurable model endpoints, tool-inventory control, budgets, and our context/session integration. If ACP does not expose a required control, make a narrow change in the pinned engine or embed the necessary core behind the same interface. Do not assume the protocol exposes all internal behavior.

Initially distribute the TUI and engine as one installable package; they may be two processes. A single executable is a later packaging optimization, not a prerequisite for a coherent product. Desktop GUI is a possible later client of the same engine boundary.

## Proposed components

This table includes both implemented components and future stages. The checked-in Cargo manifest and implementation status identify what is installed today.

| Component | Initial choice | Purpose |
|---|---|---|
| Application | Rust, Tokio, Clap | Async application, interactive and scripted commands |
| Terminal UI | Ratatui + Crossterm | Conversation, live tool status, diffs, model selection, keyboard controls |
| Agent engine | Pinned Goose, wrapped behind an `Engine` interface | Reuse sessions, agent execution and provider/tool infrastructure |
| UI/engine link | ACP Rust client over stdio | Typed events, cancellation and session communication without an open network port |
| Tool integration | Built-in typed Rust tools plus `rmcp` for selected MCP integrations | Reuse external tools while controlling the active inventory |
| Process runner | Tokio process control; PTY support where interactive programs require it | Timeouts, cancellation, exit status, bounded output and owned-process cleanup |
| Local data | SQLite, initially via `rusqlite`; files for raw artifacts | Session state, projects, tool events, model inventory and search index |
| Search | `ripgrep`, SQLite FTS5; syntax-aware chunking later | Fast exact/lexical retrieval before introducing an embedding service |
| Model/API/download clients | Existing engine adapters; `reqwest` and a suitable Hub client where needed | Endpoint probes, model metadata, resumable downloads and streaming |
| Schemas and diagnostics | Serde, JSON Schema validation, `tracing` | Validate tool contracts and explain failures |
| Tests | Rust tests, protocol fixtures, disposable local test applications | Verify behavior independently of a model's self-report |

Ratatui, ACP and MCP solve different problems: Ratatui draws the terminal; ACP connects an interface to an agent; MCP connects an agent to tools. None is itself an inference engine.

## User-facing capabilities

The first useful release should support:

- A terminal conversation with streamed text, a visible action queue, tool status, and immediate cancellation.
- A project workspace with file search/read, reviewed edits, test execution, diffs, and evidence-backed explanations.
- Sessions that resume with their objective, current state, pending actions, and artifact references intact.
- Local or remote model endpoints with arbitrary model names and visible runtime/model/context settings.
- Model discovery, downloads, local-file import, inventory, and runtime compatibility checks.
- Software and security checks against a configured project or lab, with structured findings and raw evidence.
- A noninteractive command that emits JSON/events for scripts and CI, with meaningful exit codes.

TUI views: **Chat**, **Tools**, **Files/Diff**, **Models**, **Context**, and **Findings**. Show the active model, endpoint type, usable context, current tool, elapsed time, and queued work. Offer project/session profiles so routine authorized actions do not require repeated approval.

Proposed commands, subject to implementation:

```text
alt                         Start the TUI
alt run "inspect this project" --json
alt models search qwen --variant abliterated
alt models download <repo-id> --file <artifact> --revision <commit>
alt models import <local-path>
alt providers add <name> --base-url <url> --protocol <protocol>
alt doctor
alt sessions resume <id>
alt findings export --format markdown
```

The model-search variant filter reflects publisher metadata and model-card evidence. It does not certify that a model will never refuse or that its tool behavior is reliable.

## Runtime and model compatibility

Keep inference outside the CLI initially. Support the runtime people already use, including servers on another machine.

1. **First:** OpenAI-compatible Chat Completions and Ollama, using one verified uncensored model through both where the artifact is supported.
2. **Next:** named profiles for llama.cpp, LM Studio, KoboldCpp, textgen/Oobabooga and LocalAI. A named profile is useful only after an actual tool round trip works.
3. **Then:** vLLM, SGLang, MLX-serving applications, mistral.rs, and hosted APIs where there is a concrete user need. Responses and Anthropic Messages remain explicit protocols.

ORA is the user's own runtime. Keep generic endpoint support available for it without making its internals a dependency of this project or this plan.

Each adapter/profile records protocol, base URL, optional authentication, server version, model ID, template controls, context limit, tool support, structured-output support, stream behavior, and supported parameters. Probe capabilities with harmless deterministic tools. A model list or successful chat message is not a tool-compatibility test.

No automatic cloud fallback. Users may deliberately configure fallback or escalation, with the active provider/model visible.

### Older-hardware support

Treat the GTX 1070 as a release target, not as equivalent to an 8 GB RTX GPU. It is Pascal/sm_61. The inspected [llama.cpp CUDA build logic][llama-cuda] includes the Pascal architecture in its pre-CUDA-13 path. Start by validating a suitable CUDA 12.x build targeting sm_61; do not ship a CUDA-13-only binary for this machine. Preserve CPU and, where tested, Vulkan alternatives.

Start the Pascal profile without assuming FlashAttention or quantized KV-cache kernels are available. Check actual support in the selected engine. A configuration measured on Ampere cannot be transferred just because the VRAM capacity matches.

| Hardware class | Initial model/profile target | What must be measured |
|---|---|---|
| Older CPU-only computer | Uncensored 1.7–3B model where a supported derivative is available; modest context | CPU instruction support, RAM, prompt processing, task latency and tool reliability |
| **GTX 1070 / 8 GB VRAM / 16 GB RAM** | **Uncensored 3–4B model**, compare supported Q4/Q5/Q6 quantizations; one loaded model | Actual free VRAM, buffers/cache, CUDA compatibility, 16k → 32k → 64k context progression |
| Newer 8 GB GPU | Same workflow, accelerated kernels/cache modes when verified | Whether extra context or precision improves completed-task latency |
| Larger GPU or shared-memory machine | Optional 7–9B+ models and larger windows | Separate model-specific memory, quality and performance results |

These are evaluation targets, not guaranteed fit/performance figures. System RAM and discrete VRAM are separate budgets. Leave headroom for the Linux desktop and other applications; the application should report insufficient capacity and reduce its own allocation rather than kill unrelated GPU processes. Avoid a mandatory second inference model, vector database, or large container stack for the default profile.

Package CPU-compatible host binaries and optional inference backends. Detect CPU features and GPU/backend support before choosing optimized artifacts. Publish a tested platform matrix and minimum requirements instead of promising every historical computer will work. Linux is the first validated platform; Windows/macOS compatibility follows actual testing.

## Downloading uncensored models

Provide a model browser that can search publisher labels such as `abliterated`, `heretic`, and `uncensored`, with filters for model family, size, artifact type, quantization, license, and estimated hardware needs. Allow arbitrary repositories and local paths rather than restricting users to a curated catalog.

For each candidate, show publisher, upstream/base model, revision, artifact size, template/runtime compatibility, declared license, and our test status. A publisher label and a successful download must remain distinct from a verified working profile.

The download path should:

1. Resolve the selected repository revision and enumerate the required artifact files.
2. Estimate storage and the model's memory budget, distinguishing total from active MoE parameters.
3. Download resumably into a cache with progress, cancellation, and atomic completion.
4. Verify an authoritative artifact digest when available and record provenance. Do not treat every HTTP ETag as a SHA-256 checksum.
5. Register the model and required tokenizer/template files with the selected runtime, or explain why manual import is needed.
6. Run a separately invoked compatibility check and label the result precisely.

Manage existing Ollama/LM Studio installations through their supported APIs/commands where practical. Keep raw GGUF/Safetensors/MLX files in a clear cache layout. Weight conversion, abliteration, fine-tuning, and a universal GPU installer are later features; the first version consumes supported artifacts.

Public Hub metadata does not normally need a token. If a user chooses gated/private files, accept credentials through local secure configuration and respect the repository's access requirements. Downloading weights must not automatically authorize executing arbitrary repository code.

## Making small models effective

Use the model for bounded choices and evidence interpretation; put repeatable work in deterministic tools. The initial profile exposes approximately **4–8 relevant tools at a time**, selected for the task. Do not inject the entire MCP catalog into every prompt.

The core capabilities are:

| Capability | Tool behavior |
|---|---|
| Inspect workspace | List/search/read with paths, line ranges, size limits and structured results |
| Edit code/configuration | Apply explicit patches; detect stale context; show diffs and checkpoints |
| Execute work | Run a process, read/poll output, cancel it, preserve exit status and terminate owned descendants |
| Run checks | Invoke configured test/build/lint/analyzer profiles and parse their documented results |
| Inspect an application | Bounded HTTP requests and local service checks with response evidence |
| Manage evidence | Store raw artifacts, extract relevant ranges, create findings linked to observations |

Introduce scanner integrations as reviewed, versioned tool packs: static analysis, dependency analysis, HTTP/application checks, and service inspection. Each tool pack has a small schema, prerequisite checks, time/resource limits, parser, and known-output fixtures. A generic shell remains available, but common operations should not require the model to invent command syntax repeatedly.

Validate names and arguments before execution. Return clear failure categories and a short corrective hint. Bound retries and identical-call loops. A disconnected response must not silently repeat a mutating operation. Keep tool execution/cancellation deterministic even when the model produces malformed output.

Native tool calls are the first choice. A constrained JSON action envelope is a fallback to test. Goose's experimental interpreter shim is optional because an additional model call/model can increase latency and memory. No hidden large-model dependency should make a small-model benchmark look better.

## High context: native window plus durable working state

There are two separate goals: a model can consume a large prompt, and an agent can work over a large project or long session. We should support both, while being honest about the distinction.

**Native window:** the usable context is bounded by the checkpoint, its serving configuration, and available hardware. On the reference GTX 1070, begin with 4k–8k, measure 16k, and explore 32k/64k only if actual memory and latency measurements permit. Spark-X2.5's advertised native 1M support makes it an important long-context candidate, but does not establish a practical 1M profile on this machine. Positional scaling is model-specific and must not be enabled blindly. Software around a model cannot guarantee reliable 128k reasoning from an arbitrary small checkpoint.

**Project/session memory:** retain complete artifacts outside the prompt and retrieve the relevant parts. Use:

- An explicit task ledger: objective, scope, constraints, current findings, pending actions, failures, and verification status.
- A compact repository map and exact/lexical file search, followed by syntax-aware retrieval where helpful.
- Recent conversation and tool results within a token budget.
- Durable checkpoints and summaries whose statements link back to original evidence.
- Optional embeddings later, only if they improve retrieval enough to justify their resource cost.

Example for an **8,192-token** native window: cap assembled input around **6,000 tokens**, reserve **2,000** for output, and retain a small margin. Within the input budget, allocate roughly 700 to instructions, 1,000 to active tools, 800 to task state, 1,500 to recent exchanges/results, and 2,000 to retrieved evidence. These are starting allocations to measure, not optimal constants. Different tasks may redistribute them.

Trigger compaction before exceeding the input budget. Never retain a tool call without its required result, turn a failed check into a success during summarization, or discard the only evidence for a finding. Store full logs and make them searchable instead of repeatedly sending them to the model.

Show native context use and retrieved/project memory separately in the UI. Evaluate retrieval across multiple files, retention of old decisions, contradictory evidence, and interrupted/resumed sessions. A large disk-backed memory store is not a larger native model window.

## Instructions and workflow packs

Use short instructions at the moment they are relevant. Avoid a huge universal pentesting prompt.

Instruction sources should be explicit: application/tool contracts, the user's active request/profile, and trusted project instructions such as `AGENTS.md` or `.alt/project.toml`. Workflow packs can provide additional task-specific guidance. Tool output, web pages and downloaded files remain evidence, not new instructions to the engine.

A compact initial agent contract:

```text
Work toward the user's stated objective using the available tools.
Inspect relevant project evidence before choosing an action.
Choose a small next step; use exact tool names and valid arguments.
Use tool results to decide what comes next. Do not invent execution or findings.
Keep the objective, evidence, pending work and failures in the task ledger.
Verify edits and claims with the appropriate test or observation.
Use the configured project/session permissions. Ask for missing information
when it prevents progress; do not repeatedly ask for already-authorized work.
Stop or hand control back when cancelled, blocked, or out of budget.
```

Tool packs supply procedures and deterministic checks; the prompt alone does not enforce process limits. Suggested initial packs are **code inspection**, **test/debug**, **application security review**, and **evidence/reporting**. Each has a goal, prerequisites, relevant tools, expected evidence, completion checks, and failure handling.

An abliterated model remains free to generate the requested technical analysis. Execution stays accountable to the user's selected project/lab scope and resource budget. These controls are properties of the user's tool runner, separate from a model's refusal training.

## Model choices and unresolved names

**Live-test requirement:** use an uncensored/abliterated model from the first inference test. The Heretic-published Qwen3-4B-Instruct-2507 derivative now has an inspected GGUF candidate with pinned filename, revision, SHA256 and publisher-declared Apache-2.0 license in [live-model-candidate.json](research/live-model-candidate.json). Its weights have been downloaded and verified, and small CPU tasks have been exercised with mixed outcomes; see [live evidence](LIVE_EVALUATION.md). GTX 1070 hardware fit remains untested.

All additional live test models, including fallback/interpreter models, should meet the same uncensored/abliterated selection requirement. Deterministic mocked-provider tests have no model weights and can run independently. The product may support other model types; that does not change the prototype-testing requirement.

**Spark-X2.5 is identified:** [XHToken/Spark-X2.5][spark] publishes 1.7B and 4B models. The authors describe native context up to 1,048,576 tokens, with one full-attention layer for every three sliding-window layers, and tool/agent support. The repository states Apache-2.0 for the model series and records native llama.cpp support at b10829 and Ollama support at v0.34.1. Verify the actual derivative artifact and installed runtime instead of inferring compatibility from the family name.

Put **Spark-X2.5-4B** near the top of the hardware/context evaluation queue and **1.7B** in the low-resource queue, conditional on verifying suitable uncensored/abliterated derivatives. The inspected official repository does not establish that its released models are uncensored. No such Spark derivative has been verified yet. Continue to honor the live-test requirement rather than silently substituting the standard checkpoint.

A [community deployment report][spark-community] describes a 160k profile on an RTX 3070 8 GB and explicitly separates Pascal hardware. It is useful for forming experiments, not evidence of GTX 1070 performance, and its model is not established here as uncensored. Its operational scripts were not executed. Use the official engine/model documentation and measured resource limits to construct our own supported configuration.

MiMo needs a specific variant. The official [MiMo repository][mimo] contains a **7B family**. The separately documented [MiMo-V2-Flash][mimo-flash] has **309B total parameters, 15B active parameters, and a 256k advertised window**. It is not a small-memory model simply because it activates fewer experts per token. No uncensored derivative of either was verified here. A compatible endpoint can be supported independently of whether the model fits on the target laptop.

Keep the **current MiMo releases** in the model-discovery queue as well: the official [MiMo-Code README][mimo-code] references `mimo-v2.5` and newer `MiMo-X-Pro-Preview` / `MiMo-X-Flash-Preview` offerings. These references establish names offered in that ecosystem, not downloadable weights, a particular hardware footprint, or uncensored status. The model browser should record local-artifact availability, hosted endpoint availability, and derivative verification as separate fields. The user specifically requested MiMo be included; do not omit the family simply because one variant is too large for the reference GPU.

[MiMo-Code][mimo-code] explicitly describes itself as an OpenCode fork. It is therefore outside the user's requested foundation choice, even though MiMo models themselves can still be supported.

## Delivery milestones

| Milestone | Concrete result | Completion evidence |
|---|---|---|
| **1. Engine and model proof** | Pin/build the Goose CLI target; connect an endpoint running a verified uncensored small model; validate the ACP boundary | Streaming, real tool round trip, cancellation, sessions, restricted active tool list and budget controls all work |
| **2. Branded terminal application** | Ratatui TUI plus scripted mode; packaging, configuration, sessions and diagnostics | One project can be inspected, changed, tested and resumed through our interface, with observable tool events |
| **3. Dependable tools** | Process supervisor, typed workspace/check/HTTP tools, selected MCP packs, evidence store | Timeout, malformed input, cancellation, duplicate-call and large-output fixtures pass; local lab results match independent verifiers |
| **4. Model acquisition and additional runtimes** | Search/download/import, provenance, profiles and compatibility results | Download/resume/checksum and model-import cases work; the same uncensored model workflow passes on a second runtime |
| **5. Context and small-model evaluation** | Task ledger, retrieval, compaction and context display | Long sessions resume correctly; evidence survives compaction; task success/latency/resource results are recorded on the target hardware |
| **6. Distribution** | Versioned installable artifacts, license notices, documented requirements and optional tool packs | Fresh-machine install and an end-to-end local workflow succeed on the selected launch platforms |

Implement a narrow vertical slice through these areas first: launch the TUI → select an existing local uncensored model → inspect a repository → run a check → show evidence → resume the session. Then expand model downloads and tools. Do not require every optional scanner/backend before the first useful release.

Start platform-specific work on Linux and use the GTX 1070/16 GB RAM machine as the reference hardware target. Keep portable interfaces for Linux, macOS and Windows, but test process termination, PTYs, filesystem semantics and package installation on each platform before claiming support. Cloud-machine results do not establish performance on the user's physical GPU.

## Remaining inputs and current status

- Target hardware is confirmed: Linux, GTX 1070 8 GB, 16 GB RAM. The CPU, distribution/driver versions and free disk capacity can be detected during local setup; they do not block the design plan.
- ORA requires no special work now. Spark-X2.5 is identified and researched; its exact uncensored derivative/artifact remains unverified. MiMo must also be selected by exact variant.
- Public Hugging Face metadata/cards and weight downloads now work. Alt has searched the Hub, selected a pinned artifact, downloaded it, and verified its checksum; no API credential was needed for that public repository.
- The Rust application builds and runs, including guided setup, model acquisition, managed CPU inference, and persistent sessions. Deterministic and real-Goose checks pass. Live Heretic evaluation exposes both successful tool use and instruction-following failures; retain those results and broaden evaluation before making reliability claims. No GTX 1070 benchmark has run.

[goose-distros]: https://github.com/aaif-goose/goose/blob/7debb275d36559ee4e81799362e7f638275dfed3/CUSTOM_DISTROS.md
[mimo]: https://github.com/XiaomiMiMo/MiMo/blob/3a3fe65e6081c9f340ff0d24d82c4d7a96a247c5/README.md
[mimo-flash]: https://github.com/XiaomiMiMo/MiMo-V2-Flash/blob/b4eaae40d3728657ff7f0f9397dcce3c9ab3d3b7/README.md
[mimo-code]: https://github.com/XiaomiMiMo/MiMo-Code/blob/6babeb0b98f9b4818bddf04a4331edfee04dbf85/README.md
[spark]: https://github.com/XHToken/Spark-X2.5/blob/a24ca4e6366f60b8e14211af50dfbd75d089be5c/README.md
[spark-community]: https://github.com/timothylerch/sparkx-8gb-deploy/blob/2c68987c40cd376a66cf9eb37b8ded1774cb1293/README.md
[llama-cuda]: https://github.com/ggml-org/llama.cpp/blob/50569eb87df530daff11afda229ceb9ab8e6cae8/ggml/src/ggml-cuda/CMakeLists.txt
