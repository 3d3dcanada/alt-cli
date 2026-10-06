# Product experience and acceptance criteria

Current 0.4 behavior is documented in [WORKSPACE_GUIDE.md](WORKSPACE_GUIDE.md) and tracked in [WORK_ORDERS.md](WORK_ORDERS.md). This document retains the broader product design.

Alt is for someone who can describe what they want to build but may not know
what an API endpoint, inference server, Git diff, or context window means.
The interface must provide choices and explanations where they are needed.
A command reference or configuration file is not the primary onboarding flow.

## First launch

Open a useful home screen even with no configuration, engine, model, or network.
Offer a guided connection flow with presets for Ollama, LM Studio, llama.cpp,
and another compatible server. Discover the server's models and let the user
choose one. Also offer downloaded/imported GGUF models with a managed CPU
runtime. Keep advanced addresses, credentials-by-environment-reference, and
context/budget settings editable without leaving the interface.

Show what is installed, what is connected, and what is missing separately.
An unavailable server must lead to a useful retry/edit action; it must not exit
the application. Do not silently create a paid or remote connection.

## Workspace

Use a consistent navigation rail: Home, Chat, Models, Connections, Sessions,
Settings, and Help. Keyboard shortcuts and mouse selection should both work.
Every editable field needs a visible label, cursor, focus, and hint. Support
Unicode, cursor movement, paste, multiline messages, and narrow terminals.

The home screen lets people choose a project folder, active connection, and
starting task: understand the project, build/change something, investigate a
problem, run checks, or review software security. Templates populate the message;
the user can edit before sending. Explain what each action will do.

Chat separates the conversation from tool activity. Show progress while waiting,
elapsed time, context usage when reported, and a clear cancellation control.
Approvals explain the proposed action, project, command/file, and available
choices; preserve complete details and diffs. Reject/cancel is the initial
selection. Provide a session-only trust choice, never an accidental typing key.
Tool failures and connection errors remain visible without closing the workspace.

## Models and hardware

Browse models available on a connected server, imported/downloaded local files,
and Hugging Face search results. Publisher claims such as uncensored/Heretic
must be labelled as claims. Pin download revision, byte count, license metadata,
and SHA256. Display progress, cancel/resume, disk errors, and integrity failures.
Never list a partial or invalid download as ready. Allow selecting arbitrary
compatible models, including future Spark-X and MiMo variants.

Give understandable context presets (economy, balanced, extended) with honest
memory tradeoffs. Detect host RAM/CPU and GPU information when available; do not
equate file size with required memory or cloud hardware with the user's GTX 1070.
CPU operation must work without CUDA. Starting/stopping a managed runtime must
only affect Alt-owned processes.

## Sessions and project memory

List sessions with meaningful titles, project, model, time, and preview. Search,
rename, resume, and export from the UI. Keep a short editable project brief for
goals, decisions, and next steps; include it explicitly in model context. Preserve
raw evidence separately. A failed engine must be reconnectable with saved work.
Do not claim the project brief expands the model's native context window.

## Verification required

Exercise first launch with no configuration; preset setup and model selection;
wrong address and retry; project selection; every navigation page; multiline and
Unicode input; approval/rejection; tool evidence; cancellation and forced cleanup;
session search/rename/export/resume; configuration persistence; interrupted and
corrupt downloads; and small/narrow terminal rendering. Run a real tool task with
a pinned uncensored checkpoint as soon as model CDN access permits, and record
failures as well as successes. Deterministic fixtures remain separate from model
quality results.
