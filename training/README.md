# Training handoff for Alt

October 8, 2026. **No model has been trained in this cloud.** This environment has
four CPU cores of quota, 32 GiB cgroup RAM and no NVIDIA/ROCm device. Useful 7B/9B
training is not practical here, and we do not yet have a correctness-reviewed
corpus. The [capability receipt](status/cloud-capability.json) records the probe.
Tokenizers can be exercised without weights; the
[tokenizer fixture receipt](status/tokenizer-qualification.json) is separate
from a training or inference result. [Validation status](status/validation.json)
records the executed checks and remaining gates.
Those original hardware/tokenizer receipts remain unchanged. The
[current boundary-check record](status/harness-delivery-validation.json) covers
the 46 preparation, capture, schema, encoding, resume, export and reward tests
run during harness delivery. It is separate from useful training or inference.

This handoff covers the training portions of both research passes: SM-11 and
RE-06–07, sequenced as H10–12 in the
[unified implementation plan](../docs/SMALL_MODEL_IMPLEMENTATION_PLAN.md).
[Harness research](../docs/research/2026-10-07-small-model-harness.md) and
[reasoning research](../docs/research/2026-10-07-small-model-reasoning.md) retain
the paper findings, limitations and source ledgers. SFT/distillation can change
weights; prompt/tool improvements do not train the model.

The final-pass [readiness gate](status/final-pass-readiness.json) and
[qualification handoff](../docs/FINAL_PASS_QUALIFICATION.md) retain the new
family-count, complete-attempt and exact-loader requirements. The old pending
fixtures remain pending; nothing in this pass approves or trains them.

## What is supplied

- `prepare.py`: standard-library corpus validation and train/validation export.
- `capture.py`: successful v5 native trajectories with immutable raw inference,
  oracle and current-source evidence; every capture starts pending review.
- `features.py`: native-template assistant targets with masked prompt/observation
  tokens, serialization probes, exact prefix checks and no silent truncation.
- `preflight.py`: hardware/library probe; no package installation or weight loading.
- `qualify_tokenizer.py`: opt-in native-template fixture using no weights; its
  repository/revision are explicit, and local files can be hashed for provenance.
- `readiness.py`: binds reviewed records, source/check evidence, a complete
  collection ledger, hard 32/8 family floors, exact configuration and a measured
  GPU loader receipt. Training rechecks the underlying files and current device.
- `qualify_loader.py`: plans by default; explicit `--execute` performs a disposable
  finite forward/backward pass on the exact reviewed Safetensors parent. No
  optimizer update or adapter is saved, and this is not a quality measurement.
- `train_sft.py`: optional, single-CUDA-GPU NF4 QLoRA starter using Transformers
  and PEFT, including exact dataset/config/checkpoint resume verification.
  Its data/encoding functions are tested; **GPU execution is unperformed**.
- `export.py`: validates an exact-parent export plan without Torch; opt-in
  high-precision parent/adapter merge and pinned F16/Q4_K_M conversion preserve
  receipts, failures and hashes. No adapter/export has been produced here.
- `rewards.py`: offline independently verified correctness scoring; invalid gold,
  stale source, changed tests, no-op repairs and unsupported claims are excluded.
- `configs/sft-7b.json` and `sft-9b.json`: explicit checkpoint placeholders and
  starter hyperparameters. Both reject execution until model identity and rights
  are supplied. These are deployment-size templates, not architecture certifications.
- `examples/`: record and task-family registry templates, not usable training data.
- `tests/`: weight-free checks of contamination, evidence, tool exchanges and loss
  masks. Fixtures are synthetic and never an approved production corpus.
- [Checkpoint notes](CHECKPOINTS.md): inspected 7B/9B candidates, including the
  important modern-architecture qualification gap for the MiMo 9B derivative.

The driver consumes reviewed data. It does not generate teacher answers, upload
anything, call an external judge or publish weights. All model actors in live
data collection and evaluation must be explicitly selected uncensored/abliterated
checkpoints. Human-reviewed examples are also supported. Training helpers are
separate from Alt's runtime. The current source executable has the harness
workflows in the [usage guide](../docs/SMALL_MODEL_USAGE.md); it does not train
weights during an ordinary conversation.

## 1. Choose the exact training checkpoint

Record your current 7B/9B Q4 GGUF repository/revision/hash, derivative ancestry,
runtime, template/parser and settings. Locate the matching **Hugging Face training
checkpoint**, usually Safetensors plus config/tokenizer. A quantizer's `base_model`
link is a lead; confirm the exact source and applicable terms. If the derivative
exists only as GGUF, get its original HF checkpoint or deliberately choose a new
student and qualify it separately. Do not silently substitute the unmodified
upstream checkpoint or train an adapter against different abliterated weights.

Q4 GGUF inference and NF4 QLoRA training are different representations. The
starter loads an HF checkpoint into four-bit NF4, keeps its base weights frozen
and trains LoRA matrices. It does not directly optimize your GGUF. Exporting the
result to Q4 happens after training and high-precision evaluation.

Model rights, dataset rights and training-code licenses are distinct. The inspected
Open-AgentRL-3K/30K cards do not establish reuse/redistribution permission; this
handoff does not download those datasets. Nemotron's noncommercial and LFM's
custom terms need separate handling. Review derived/teacher output rights before
including data. Keep private or credential-bearing traces out of a distributable
corpus. The preparation flags record that review; they cannot perform it for you.

## 2. Freeze task-family partitions before collecting traces

Copy `examples/split-registry.template.json` into an ignored `.alt-training/`
working directory. Assign a stable family/lineage ID, grouping variants of the
same problem, repeated attempts and closely related repositories together. Every
family belongs to `train`, `validation` or `test`. Do not split random messages
from one task across partitions.

Training sees only train. Validation may select hyperparameters/checkpoints but
never supplies gradient targets. Test stays sealed from collection, prompts,
skills, teacher examples and model selection until the final frozen evaluation.
The preparation command refuses test records entirely. It checks declared family
boundaries and exact transcript duplicates; reviewers must catch semantic near
duplicates, paraphrases, shared solutions and repository lineage.

Known historical held-out Alt tasks are now seen development material. Make new
held-out families. The 23 historical passing attempts include incomplete repairs
under broader checks; do not bulk-convert them to successful demonstrations.

For a first SFT pilot, collect at least **32 independent training families and
eight validation families**, as required by the supplied configs. This is a pilot
floor, not evidence that 40 families suffice for a good agent. Aim next for varied
50–200+ reviewed tasks across languages and failure types, expanding when the
actual learning curve calls for it. Count families and assistant targets separately.

## 3. Capture actual, reviewed trajectories

Keep raw Alt JSONL events, provider/engine requests when available, source snapshots
and checks. Extract a normalized transcript **without rewriting tool results**:

```json
{
  "messages": [
    {"role": "user", "content": "The actual authorized task"},
    {"role": "assistant", "content": null, "tool_calls": [
      {"id": "call-1", "type": "function", "function": {
        "name": "read", "arguments": {"path": "src/example.py"}
      }}
    ]},
    {"role": "tool", "tool_call_id": "call-1", "content": "Actual recorded output"},
    {"role": "assistant", "content": "Actual recorded final response"}
  ],
  "tools": [
    {"type": "function", "function": {
      "name": "read", "description": "Read a file",
      "parameters": {"type": "object", "properties": {
        "path": {"type": "string"}
      }, "required": ["path"]}
    }}
  ]
}
```

This illustration is not real training evidence. Preserve actual tool names,
contracts and outputs from the captured run. Normalize an API's JSON-string
arguments to objects for HF templates, while retaining the original raw trace.
Every call needs its real result; no orphan results, unfinished exchanges or
invented “passed” outputs. `prepare.py` checks structural exchanges and required
arguments and the supported native JSON Schema subset, including types,
required/additional properties, enums, bounds, arrays and local references.
Unknown keywords and external references require separate qualification rather
than silent acceptance. Schema validity does not establish that code is correct.

Use `examples/trajectory.template.json` to create a JSONL record. Its `source`
points to the raw trace, normalized transcript and exact tested source snapshot
under an evidence root. Each has a SHA256. A snapshot can be an immutable archive
or source manifest; checks must bind the same snapshot identity and actually test
its contents. The normalized transcript must equal the record's messages/tools.

Wrap each independently executed behavioral check in a recorded observation:

```json
{
  "schema_version": 1,
  "name": "behavior-contract-v1",
  "kind": "independent_behavior",
  "passed": true,
  "exit_code": 0,
  "tests_run": 12,
  "source_sha256": "SHA256_OF_ACTUAL_TESTED_SNAPSHOT",
  "raw_output_path": "evidence/attempt/behavior.stdout",
  "raw_output_sha256": "SHA256_OF_ACTUAL_RAW_OUTPUT"
}
```

Populate these fields from execution, not a model's answer. Preserve the oracle,
invocation, dependencies and coverage beside raw output so the reviewer can
re-run it. Missing/invalid gold, altered tests, early exits and stale snapshots
are not successful checks. A compiler/format pass alone is insufficient. Hashes
prove byte identity; they do not prove truthful provenance or comprehensive tests.

The reviewer approves source correctness, actual observations, rights, privacy
and partition membership. Prefer concise evidence-grounded decisions and actual
native operations; compare mixed lengths later. Recorded public reasoning can
be retained if the native template supports it. Do not fabricate hidden reasoning
or label an inaccurate explanation correct because the final answer passes.
Successful recovery from a real failed check is useful; the failure remains a
failure inside that trajectory. Standalone failed tasks need a separate future
preference/RL design and are refused by this successful-trajectory pipeline.

For a model actor, record its immutable repository revision, exact weight artifact
or shard-manifest SHA256, precision/quantization, native-template SHA256 and runtime
build. A repository containing several GGUF variants is not an exact actor identity
by itself. Preserve sampling, context/reasoning limits and actual requests beside
the raw trace. No unknown actor or hidden teacher is an accepted substitute.

`capture.py` exports complete, independently successful v5 attempts into pending
records, as described below. It uses actual provider exchanges and source/oracle
receipts; arbitrary database conversations and compacted historical ACP logs
are not automatically converted into HF-ready demonstrations. `prepare.py` still
requires explicitly reviewed records.

The later repair follow-up now includes
[two actual pending native trajectories](../docs/research/2026-10-07-repair-results/pending-training/),
one each from independently successful Python feature and two-file repairs by
the pinned MiMo 9B Q4 Heretic model. Their actor, exact template/runtime, original
oracle, source snapshots and raw provider exchanges are retained. Both are
synthetic acceptance projects; all rights/privacy/correctness/split review flags
remain false. They are examples to inspect and review, not an approved dataset.
Two families cannot meet the 32-train/eight-validation pilot floor. Nothing has
been trained or exported from them.

## 4. Prepare and validate data

From the repository root, after creating the records/evidence:

```bash
python3 training/preflight.py --output .alt-training/pc-capability.json
python3 -m unittest discover -s training/tests -v
python3 training/prepare.py \
  --records .alt-training/reviewed-trajectories.jsonl \
  --split-registry .alt-training/split-registry.json \
  --evidence-root .alt-training \
  --output .alt-training/prepared-v1
```

The export writes `train.jsonl`, `validation.jsonl` and a manifest with hashes,
counts and family IDs. It refuses malformed evidence, duplicate conversations,
unapproved data, test-family records and an existing output directory. A failed
write without a manifest is incomplete; keep/remove that partial directory
deliberately and retry into a new path. `--allow-fixtures` marks the **entire**
export fixture-only; `train_sft.py` refuses such exports. Neither command scans
your machine for data or executes transcript tools.

## 5. Qualify a training machine and stack

Use a CUDA training environment with the exact chosen architecture supported.
A modern 24 GB GPU is a reasonable **qualification starting point** for these
7B/9B QLoRA pilots, not a measured requirement or fit guarantee. Some smaller
cards may work with shorter sequences and selected adapters; measure actual
activation/optimizer peaks before committing to a run. The GTX 1070's 8 GB Q4
inference capacity does not establish training fit, BF16 or FlashAttention support.
Keep its deployment test separate. CPU-only data preparation works here and on
the PC; meaningful CPU-only 7B/9B fine-tuning is not the proposed recipe.

The reference starter targets **text-only AutoModelForCausalLM architectures
supported by Transformers 4.57.1**, using eager attention, FP16/NF4 and one visible
CUDA GPU. It does not enable arbitrary remote model code. Spark's custom type and
newer Qwen3.5/MiMo conditional-generation architectures need a separately qualified
stack/loader; see [checkpoint notes](CHECKPOINTS.md). Failure to support an
architecture is not solved by loading a different model silently.

Use an isolated environment, install a PyTorch build compatible with the selected
GPU/driver, then the reference packages. The pins are API references checked
against published sources; **the GPU combination has not run here**, and top-level
pins are not a complete environment lock. CUDA local-version builds may require
updating the explicit reference check and recording that qualified build.

```bash
python3 -m venv .alt-training/venv
.alt-training/venv/bin/python -m pip install -r training/requirements-sft.txt
cp training/configs/sft-7b.json .alt-training/sft.json
# Use sft-9b.json instead for a supported 9B training checkpoint.
```

Edit `.alt-training/sft.json`: exact HF repository, immutable commit, uncensored
selection, reviewed model terms and actual native template controls. Do not infer
a thinking toggle from model size. Save GPU/driver/CUDA/package versions, model
config and tokenizer hashes with the run. Qualify a tiny supervised step on the
chosen hardware before a larger budget; its completion is a stack smoke test,
not a stronger-model result. Watch measured memory and stop on incompatibility.

## 6. Qualify tokenization before loading weights

An optional synthetic fixture can run before you have a reviewed corpus:

```bash
python3 -m venv .alt-training/tokenizer-venv
.alt-training/tokenizer-venv/bin/python -m pip install -r training/requirements-tokenizer.txt
.alt-training/tokenizer-venv/bin/python training/qualify_tokenizer.py \
  --repository Goekdeniz-Guelmez/Josiefied-Qwen2.5-7B-Instruct-abliterated \
  --revision a6b1fb11f7096463d7e0792c36543869a83f2ed7 --uncensored \
  --output .alt-training/template-fixture.json
```

This explicitly chosen **tokenizer-only example** does not select a training or
runtime model. Use your own pinned candidate instead when it has tokenizer files.
`--local-tokenizer /path/to/verified/tokenizer-folder` avoids network acquisition;
the receipt hashes local files, whose identity must match the source ledger.
This synthetic fixture cannot establish model quality or substitute for the real
reviewed-corpus pass below. With your configured training stack and real data:

```bash
.alt-training/venv/bin/python training/train_sft.py \
  --config .alt-training/sft.json --dataset .alt-training/prepared-v1 \
  --tokenize-only --output .alt-training/tokenizer-receipt.json
```

For each assistant decision, the encoder renders its prior conversation as the
native generation prompt and its completed response with the same tools/settings.
The prompt tokens must be an exact prefix of the completion. Everything before
the current assistant target, including earlier assistant messages and tool/user
observations, has label `-100`. Padding stays masked. No template `generation`
annotations are required. Native call name/arguments and any recorded
`reasoning_content` must affect serialization; dropped fields are rejected.

Prefix mismatch may indicate BPE boundary behavior, a template's thinking prefill
or inconsistent serialization. Inspect and qualify a matching native format;
do not trim tokens until a test passes. Overlong examples fail without truncation.
Re-collect bounded real evidence or deliberately increase the context after memory
qualification. A 2K training sequence is a starter budget, not a 2K model-context
limit or a way to extend its pretrained native window.

## 7. Qualify readiness, then run the pilot

Keep a complete collection ledger using
[`examples/collection-ledger.template.json`](examples/collection-ledger.template.json).
Declare every planned attempt before collection and retain failed, interrupted and
unperformed entries. The ledger's `plan` points to the separately frozen
[`collection-plan.template.json`](examples/collection-plan.template.json), with
its actual declaration time and SHA-256. Its slot IDs, families and splits must
match the ledger exactly; omitted failures cannot disappear from accounting.
Each performed outcome references a hashed actual attempt receipt through
`evidence.path` and `evidence.sha256`, relative to the evidence directory. See
[`collection-attempt.template.json`](examples/collection-attempt.template.json).
The receipt binds the plan hash, actual start time, outcome, actor and the exact
raw trace, transcript, source snapshot and check hashes. A `passed` label alone
is insufficient. Templates are deliberately unusable until actual measurements
and review supply the missing identities.

A reviewed record needs `collection.attempt_id` and
`collection.ledger_sha256` linking it to a passing attempt of the same family and
split. Freeze that ledger before preparing the reviewed corpus; adding attempts
requires a new version and newly bound records. Final held-out families cannot be
used for corpus, prompt or adapter selection. Do not relabel them.

On a suitable, explicitly selected CUDA machine, inspect the exact pinned parent
and configuration, then qualify the loader. The default invocation writes a plan
without importing Torch or downloading weights. Add `--execute` to actually load
the parent and measure the backward pass. The test establishes loader compatibility
on that GPU, not fit for a complete 2K training sequence or improved behavior.

```bash
CUDA_VISIBLE_DEVICES=0 .alt-training/venv/bin/python training/qualify_loader.py \
  --config .alt-training/sft.json --output .alt-training/loader-001 --execute
.alt-training/venv/bin/python training/readiness.py \
  --records .alt-training/reviewed.jsonl --registry .alt-training/split-registry.json \
  --evidence-root .alt-training/evidence --ledger .alt-training/collection-ledger.json \
  --config .alt-training/sft.json --loader-receipt .alt-training/loader-001/loader-receipt.json \
  --output .alt-training/readiness-001.json
```

The readiness command returns exit 2 with concrete blockers when prerequisites
are missing. It never trains. Zero reviewed families and no CUDA remain the
cloud result. Once this receipt passes and `prepare.py` has exported the exact
same records and registry, start the explicitly authorized pilot:

```bash
CUDA_VISIBLE_DEVICES=0 .alt-training/venv/bin/python training/train_sft.py \
  --config .alt-training/sft.json --dataset .alt-training/prepared-v1 \
  --train --readiness-receipt .alt-training/readiness-001.json \
  --output .alt-training/sft-run-001
```

Defaults: NF4 double quantization; rank 16/alpha 32; batch one, accumulation 16;
2K maximum sequence; learning rate 1e-4; 100 optimizer updates; validation/checkpoint
every 25 updates. These are bounded pilot hypotheses, not optimized settings.
Repeated context becomes separate masked samples for each assistant target.
Report targets/tokens/families, effective updates, elapsed time and peak memory.
Keep concise and mixed-length arms matched by a declared update/token budget.

The driver writes package/model/template/dataset identities, run status, adapter
hashes and CUDA memory peaks. It retains failed runs and refuses an existing output.
Validation loss can select a pilot checkpoint; the resulting status deliberately
remains `training_finished_unqualified` until behavioral evaluation. For recovery,
preserve Trainer checkpoints and the prior receipt. Supply `--resume-from
/path/to/checkpoint-N --resume-receipt /path/to/prior/run-receipt.json`, keeping
the exact model, dataset and config and selecting a new output directory. The
driver verifies every recorded checkpoint file before loading weights and again
before resuming. New architectures still need separately qualified trainers.

## 8. Evaluate, merge and export Q4 separately

Freeze the before/after evaluation before seeing results. Use fresh task families,
native tool round trips, argument correctness, editing, recovery and truthful
completion checks. Keep equal total effort and exact runtime/template/sampling;
report pass@1, selected-best-of-k, false success, latency and memory separately.
Evaluate ordinary instruction following and the desired uncensored workflow too.
Training can regress either; lower loss is not sufficient.

For a supported architecture, reload the **exact original derivative** in
FP16/BF16 with its recorded revision; apply the saved PEFT adapter with
`PeftModel.from_pretrained`, then `merge_and_unload()` and save Safetensors plus
the qualified tokenizer/template. Do not merge into an already NF4-quantized base
and assume equivalence. Full-precision merge may require substantially more
host/device RAM than the training/inference artifact; 16 GB PC RAM may not suffice.

Use the model-compatible pinned llama.cpp conversion tool for the saved model,
then its `llama-quantize` to produce Q4_K_M. Check architecture support and exact
CLI options at that pinned build; custom/multimodal/hybrid models need their own
conversion recipe, including required companion artifacts. Record converter and
quantizer hashes, source/adapter/output hashes, applicable model terms and any
template changes. First compare untrained versus trained high-precision results,
then measure quantization as a separate factor. Finally import the selected GGUF
into Alt and qualify load, parser/tool behavior, 4K/8K context, cache/offload and
actual RAM/VRAM on the GTX 1070. There is no promoted trained artifact today.

The export helper validates this plan without loading weights by default:

```bash
python3 training/export.py --config .alt-training/sft.json \
  --run .alt-training/sft-run --converter /path/to/llama.cpp/convert_hf_to_gguf.py \
  --quantizer /path/to/llama-quantize --output .alt-training/export-new
```

Only append `--execute --device cuda:0` after the exact architecture, merge RAM,
converter commit and output rights are qualified. CPU merge is explicit too and
can require much more than 16 GB RAM. The helper reloads the high-precision
matching parent and never merges into the NF4 training copy. The resulting
`exported_unqualified` receipt is not a deployment or quality promotion.

## 9. Follow-on reasoning training

After useful SFT, compare SOD/OPD and verified-reward RL with the same student,
permitted teacher, task mixture and partition boundary. SOD is particularly
relevant to tool-step recovery, but its inspected published recipe uses eight
96 GB H20s; do not transfer that hardware recipe to an 8 GB Pascal card.
DeepScaleR/Open-RS/VibeThinker competition scores are not Alt agent outcomes.

Test rewards before training: invalid gold/evaluation means **excluded with
counts**, not reward one; wrong answer, no-op, changed tests, timeout and unsupported
success cannot pass. Correctness precedes formatting/length efficiency rewards.
Keep rollout/teacher/training costs and sampling coverage separate from pass@1.
All teacher/router/judge model identities remain explicit and uncensored. A teacher
is a training dependency; the exported local student should not need it at runtime.

`python3 training/rewards.py --help` describes the offline scoring input. Keep
gold and candidate independent-check receipts, actual current source, unchanged
oracle inputs, raw outputs, rights review and recorded cost. Excluded rows have
`reward: null` and a reason. Valid wrong answers receive zero; known efficiency
adds a bounded bonus only after correct behavior. This scorer does not run an RL
optimizer. SOD/OPD/RL remains gated on a useful SFT baseline and qualified hardware.

## Capture current harness trajectories

Run the v5 live evaluator on development families. A passing external oracle alone
does not approve training: inspect actual actions, source correctness, source/output
rights, privacy and family splits first. Capture a candidate with:

```bash
python3 training/capture.py --attempt /path/to/v5/attempt \
  --actor .alt-training/actor.json --source .alt-training/source.json \
  --family python-feature --split train --output .alt-training/pending-new
```

Actor metadata must identify the evaluated uncensored artifact hash; source
metadata declares repository, revision and applicable license. The helper rejects
sealed or renamed families, changed source/oracle/evidence, interrupted provider
streams and omitted native exchanges after context compaction. It retains raw
ACP/provider exchanges and writes `trajectory.pending.jsonl` with every review
flag false. Review it before using `prepare.py`; do not train the pending file.

## Validation boundary

Committed tests exercise real file hashes, source/check consistency, split
boundaries, duplicate/orphan data, loss masking, padding, dropped tool arguments
and output preservation. A separate tokenizer fixture uses an inspected 7B
abliterated checkpoint's tokenizer without model weights. Training source/API
inspection is recorded in [the handoff source ledger](status/sources.json).

GPU allocation, backpropagation, optimizer behavior, adapter merge, Q4 conversion,
trained-model gains and physical GTX 1070 performance are **unperformed**. The
roadmap keeps those as explicit acceptance work; this repository does not contain
trained weights, a production corpus or a claim of stronger reasoning yet.
