# Command-line guide

Run `alt --help` or `alt COMMAND --help` for the exact options in your build.
`alt` without a subcommand opens the TUI. Run project commands from the project
folder, or select the project in Settings. `--data-dir`, `--profile` and `--engine`
select saved state, a named model profile and an engine executable.

The current source also provides [small-model inference, skills, candidates and
experimental workflows](SMALL_MODEL_USAGE.md). Use that guide for the new commands;
older published executables can have fewer subcommands.

## Connect and inspect

```bash
alt init --provider ollama --model YOUR_MODEL --uncensored
# Alternative for a new configuration:
# alt init --model YOUR_MODEL --endpoint http://127.0.0.1:1234/v1 --uncensored
alt doctor
alt hardware
alt build-info
```

`init` never overwrites existing configuration. Use Connections in the TUI to add
or edit profiles. Ollama uses its server root; OpenAI-compatible endpoints normally
include `/v1`. Mark `--uncensored` only when that is the artifact you selected.
It records a user/publisher claim; it does not alter the model.

`doctor` checks inventory and the engine protocol without inference. `hardware`
reports detected resources and guidance; `build-info` prints embedded provenance
without opening saved state. An external server's native context must be set in
that application; changing Alt's budget does not configure the server.

## Get models and runtime components

```bash
alt models
alt models search 'Qwen3 4B heretic'
alt models files mradermacher/Qwen3-4B-Instruct-2507-heretic-GGUF
alt models download PUBLISHER/REPOSITORY --file EXACT_FILENAME.gguf --use
alt models import /path/to/model.gguf --uncensored --use
alt models local
alt install engine
alt install runtime
```

The repository and filename placeholders must be replaced with actual Hub choices.
Downloads verify published SHA256 metadata, and imports preserve the original
file. Use the Models UI for guided selection, pause/resume and cache management.
`HF_TOKEN` is optional for repositories your Hub account can access; accept any
gating terms on Hugging Face first. Authenticated Ollama inference is not supported.

## Run a task

Start with `alt practice` to create a fresh example in Alt's data folder. Open the
TUI and choose **Try a practice project** on Home for the repair/check/undo guide.
The four Python checks are pinned outside the editable example. An existing
project is preserved, and each new lesson gets its own folder. Home's **Prepare
project checks** offers commands discovered from your project's actual manifests;
review a suggestion before saving or running it.

```bash
alt run 'Inspect this project and explain its test commands'
alt --access trusted run 'Run the documented checks and explain failures' --allow-tools --json
alt plain
alt sessions
alt tui --resume SESSION_ID
alt run 'Continue checking the project' --resume SESSION_ID
alt export SESSION_ID > conversation.jsonl
```

For the compact small-model interface, choose `alt tools compact` (scalar text)
or `alt tools compact-lines` (one source line per array item). `alt tools all`
restores the original tool/context interface. These choices are independent of
Full/Guided access. Set workflow with `alt workflow host` and output allocation
with `alt inference --output-tokens 1024`; use the
[usage guide](SMALL_MODEL_USAGE.md) for model-specific reasoning and context settings.

Headless requests deny tool permission unless `--allow-tools` is supplied. Access
policy is separate: `--access trusted` selects Full access; other values are
`guided` and `review-only`. The TUI uses its saved Settings choice. Full access
can run arbitrary commands with normal host permissions. `--json` writes events
to stdout, with session IDs and diagnostics on stderr. The default timeout is
600 seconds; change it with `--timeout SECONDS`. Cancellation, failure and timeout
exit nonzero. A successful process exit or natural model turn is not proof that
the requested behavior passed checks.

`plain` offers a text conversation with per-action approval; `/quit` leaves it.
Exports and logs can contain project data. Choose what to share when reporting a bug.

## Configure evidence

```bash
alt task configure-check build -- cargo build --locked
alt task contract build --kind build
alt task require builds --check build --description 'The project builds successfully'
alt task verify --run
alt task status
alt task changes
```

This proves the configured build result, not behavioral acceptance. For a separately
maintained assertion that writes Alt's structured JSON report:

```bash
alt task configure-check acceptance -- python3 /trusted/acceptance.py
alt task contract acceptance --kind tests --format json \
  --report .alt-check-results.json --assertion /trusted/acceptance.py
alt task require behavior --check acceptance --description 'The requested behavior and regressions'
alt task verify --run
```

The assertion runs against the disposable source copy and writes to
`ALT_CHECK_REPORT`. See [Verification contracts](VERIFICATION_CONTRACTS.md) for
JSON/JUnit/TAP formats, assertion pinning, coverage and evidence freshness.
Guided isolation must be available, or explicitly choose your intended access mode.

## Terminal jobs and selected tools

```bash
alt --access trusted jobs start 'npm run dev' --name 'Development server'
alt jobs list
alt jobs health JOB_ID http://127.0.0.1:3000/
alt --access trusted jobs attach JOB_ID
alt jobs stop JOB_ID
alt jobs restart JOB_ID
alt tools all
alt tools coding
alt --access trusted packs run http --input '{"url":"http://127.0.0.1:3000/","contains":"Ready"}'
```

The commands assume that project and local service exist. Ctrl+] detaches from an
attached terminal; Ctrl+C belongs to the attached program. Jobs can have explicit
lifetimes and timeouts; inspect `alt jobs start --help`. Tool focus options are
`all`, `inspect`, `coding` and `terminal`. Coding exposes named checks but omits
the arbitrary terminal schema; configure checks or choose another focus when needed.
Focus does not change access policy. TUI Settings also manages external MCP tools.

## Measure and recover

```bash
alt runtime --gpu-layers 0 --threads 2 --batch 128
alt runtime --thinking false
alt runtime --thinking-default
alt evaluate
alt benchmark
alt qualify --contexts 2048,4096,8192 --repeats 1
alt state backup /path/outside-alt-state/backup.tar.gz
alt state restore /path/to/backup.tar.gz /path/to/new-restored-state
```

Live evaluation commands require an explicitly selected uncensored/abliterated
model. Only templates supporting `enable_thinking` honor the reasoning override;
model default supplies none. Qualification keeps the model/settings unchanged and
retains failures. Throughput and a tool probe do not establish coding reliability.
Backup/restore scope and version rollback are explained in [Installation](INSTALLATION.md).
