# Using the small-model harness

These additions are included in
[v0.6.0-beta.2](https://github.com/3d3dcanada/alt-cli/releases/tag/v0.6.0-beta.2).
Read the [compatibility record](COMPATIBILITY.md) and
[delivery record](SMALL_MODEL_DELIVERY.md) for what was tested and what remains
unqualified. A useful tool harness does not guarantee that every model can solve
every task.

## Start with the model you already use

Open Alt, choose your project and connect your existing Ollama, LM Studio,
llama.cpp, ORA or compatible endpoint. Alternatively import your own GGUF from
Models. Your 7B or 9B Q4 model stays selected. Neither effort nor a skill switches
the model or introduces a hosted teacher.

On Linux with 16 GB RAM and an 8 GB GTX 1070, start with a measured 4K or 8K window.
Use CPU or partial offload if needed. The cloud's successful MiMo 9B native-tool
probe used CPU, 4K context and a specific pinned artifact; it does not establish
Pascal kernel support, VRAM fit or coding reliability. GPU layers require your
own compatible runtime. See [PC testing](PC_TESTING.md).

The later [repair pilot](research/2026-10-07-repair-results/README.md) used that
same 9B Q4 artifact at **8K**, with 1,024 output tokens, a 128-token reasoning
allocation and ten-minute CPU deadlines. Both original and compact interfaces
passed 3/4 tasks; compact used less input and its successful repairs took less
time in this small pilot. Some earlier four-minute and five-minute CPU trials
timed out before corrected calls completed. Other attempts failed through invalid
arguments or incorrect logic. Context, output reserve and loading/preparation cost must be measured
together; a successful short probe does not qualify a coding configuration.

## Give the model clear settings and actual evidence

In Compact, a follow-up such as “continue” retains the complete original task
request as context. Your latest message takes precedence, and source retrieval
uses both messages. If the stored request and pinned requirements exceed the
context budget, Alt reports the limit instead of silently dropping requirements.
Start a new conversation for a separate task.

Output and reasoning reserves share the native context window with conversation
history and tools. Increasing output can leave less room for accumulated input.
An 8K Rust development trial stopped at ten issued requests because input exceeded
its reserve; an explicit 16K trial reached twelve requests but still failed the
unchanged check. More room and reasoning output alone did not establish a repair.

**Alt+6 → Tool focus → Compact** selects smaller current-source context and simple
text edits. The host supplies complete observed source spans with short handles;
the model sends a path, handle and replacement text. Failed or stale writes are
refused. A successful edit returns the actual updated source and its new handle
for correction after a failed check. Pinned requirements and current check
status take priority over historical notes.

Edit feedback identifies previously extracted definitions missing from the
current source. This is a review hint; intentional deletion and renaming remain
available. Parser observations are separate from actual compiler and behavioral
checks. Check traceback paths under `/tmp` refer to completed disposable copies;
read and edit the original project with relative paths. Raw diagnostics remain
available in evidence.

**Compact with line-array edits** is an explicit alternative for parsers that
handle arrays reliably. It takes one literal source line per array item. The
scalar format avoids nested JSON-array parsing in native tool templates. Neither
choice changes the selected model, access mode or configured check requirements.
Keep **All native tools** to use the original context and edit interface.

The equivalent commands are `alt tools compact`, `alt tools compact-lines` and
`alt tools all`. Choose the workflow separately with `alt workflow host` or
`alt workflow model-plan`. These are explicit choices, not automatically promoted
quality presets. See [repair work orders](REPAIR_WORK_ORDERS.md) for the tests and
the database migration note.

Press **Alt+6 → Model and runtime settings**. The menu offers:

- **Model output and sampling:** total output, thinking allocation, action
  headroom and sampling. Output includes reasoning and tool arguments. Blank
  optional fields mean the selected server's defaults. Thinking controls require
  an actual supported template/parser. A non-thinking model gains no thinking
  mode from this setting.
- **Task workflow:** keep a model-written plan or explicitly choose **Host guides
  each step**. Host mode saves a host-authored plan and returns the next decision
  from current source and actual check results. Hypotheses remain labelled.
- **Task skills:** choose one short procedure. The host otherwise offers a small
  metadata shortlist. Python repair, Rust diagnostics, JavaScript setup, browser
  checks, dependency review and configuration/data have executable helpers.
  Helpers require their declared software and your chosen access.
- **Test native model tools:** stream a call, execute a harmless host probe and
  require the model to return a fresh value supplied only in the tool result.
  Read a failed result before
  spending time on a larger coding task.

Settings apply to a new connection. Unsent messages remain in the composer.
Owned inference records the full rendered chat/tools token count when the
selected runtime exposes its template and tokenizer endpoints. Requests exceeding
the selected input reserve are rejected before inference. External tokenization
stays unknown; configure and inspect that server's native window separately.

Chat shows observed waiting/receiving stages, input counts when measured, output
reserve and remaining connection allowances. Waiting includes server queueing
and prompt processing; it is not a reasoning measurement. Interrupted requests
remain charged when final usage is unknown. **Allowance → Add a finite
allowance** requires confirmation and preserves spent costs. It neither changes
the model nor sends another request. Limits span messages on that connection;
an explicit reconnect starts a new connection allowance.

After changing output/context settings, use **Allowance → Apply saved allocation
and reconnect** to update the existing task's saved allocation. Review the
changes before confirming. The model, provider, original goal and transcript
remain the same; old receipts remain evidence. This is useful when an old saved
session still has the previous input/output reserves.

Failed checks supply observed case names, diagnostic locations and fresh source
handles. Repeated failures on unchanged source and revisited failed revisions
produce a different recovery hint. These are bounded observations, not a proven
diagnosis or an automatically supplied solution. Full-access commands remain
available. A passing build still does not replace the task's behavioral checks.

Ask for a concrete change and prepare the project's real check command from Home.
Use **Task** to describe its evidence contract and pin an independent assertion
when you need behavioral acceptance. Build success, model prose and fluent scores
cannot stand in for the requested behavior. **Context** shows bounded source
outlines, import candidates and retrieved snippets. **Task evidence** exposes
additional pages of captured output; the short diagnostic packet is a preview.

Native file reads return revision-bound range and symbol handles. The model can
replace a handle instead of reproducing a large exact old string. Stale handles
and concurrent user edits fail without overwriting the new file. Unsupported
syntax still has an exact-text fallback. File changes keep diff/checkpoint/undo.

## Explore alternatives under one allowance

Choose **Explore verified candidates** in the same settings menu. Describe the
goal, then select **Quick**, **Careful** or **Thorough** for up to one, two or three
serial source candidates. Select Standard, Modest, Extended or your own shared
time, generated-token and request allowance. More candidates divide that same
allowance; they do not each receive a fresh budget. Model loading and actual
checks count toward the time limit. Unknown/interrupted generation is charged
conservatively. A verified candidate stops further exploration; two attempts with
unchanged source stop for lack of progress.

This feature requires configured **required Tests checks with independently
pinned assertions**. Each candidate gets a separate source copy, task state,
ancestry, diff and raw inference evidence. Original files change only when you
review and explicitly apply an eligible candidate. Applying rejects source
conflicts and keeps checkpoint undo; rerun checks on the original project afterward.
Permission changes and binary mutations stay in the candidate copy for manual
review rather than automatic promotion.

Source copies are not OS isolation. With **Full access**, terminal/network tools
retain your normal host permissions and can affect external files, services or
the original project. Guided check isolation continues to require Bubblewrap.
Tool focus, execution access and effort remain independent choices.

Advanced CLI equivalents:

```bash
alt inference
alt inference --output-tokens 1024 --action-headroom 256 --temperature 0.2
# For a qualified thinking llama.cpp checkpoint only:
alt inference --reasoning-tokens 512
alt workflow host
alt skills list
alt skills use python-repair
alt --access trusted skills run python-repair build
alt qualify-tools --timeout 600
alt --access trusted candidates run 'Repair the requested behavior' \
  --effort careful --seconds 600 --generated-tokens 8192 --requests 12
alt candidates list
alt candidates show RUN_ID
alt --access trusted candidates apply RUN_ID 1
```

`candidates run --resume RUN_ID` needs the same goal, limits, profile, preferences,
engine, executable and source baseline. Interrupted allowances remain charged.
Failed candidates, raw output and diffs remain on disk. `alt inference --reset`
clears explicit sampling/reasoning/shared-limit overrides and restores the legacy
per-response allocation; it does not change the checkpoint.

## Keep experimental improvements reviewable

Offline optimization produces a draft, using only development feedback and an
explicit permitted uncensored optimizer. `scripts/optimize_instructions.py --help`
describes its inputs. Trial a draft with `alt run ... --instruction-draft FILE` or
`scripts/live_acceptance.py --suite v5 --instruction-draft FILE`. A trial affects
only that fresh invocation. Import the draft and provenance using
`alt instructions import BODY METADATA`; review its diff and separate validation
reports before `alt instructions review ID RECEIPT` and `alt instructions use ID`.
`alt instructions clear` restores the core instructions; earlier reviewed versions
can be selected again. The TUI's **Reviewed instructions** menu uses the same gates.
Sealed final-test families cannot select instructions.

`alt analyze FILE QUESTION --experimental` is a bounded recursive-analysis
experiment, with explicit calls/depth/time/generated-token limits and quoted
source offsets. It selects bounded relevant spans and reports omitted bytes.
Summaries remain unverified. A reviewed exact-source retrieval-gap receipt is
needed to claim the retrieval gate was met; the controller's protocol fixture
does not establish a reasoning gain or expand native context.

For weight training, use the [training handoff](../training/README.md). Trace
capture starts pending review. Standard QLoRA trains the exact matching HF
checkpoint, not a Q4 GGUF. Useful SFT, GPU execution, adapter merge, Q4 conversion,
distillation/RL and held-out quality remain separate gates.
