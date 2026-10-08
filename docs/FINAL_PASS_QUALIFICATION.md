# Final-pass qualification and training handoff

October 8, 2026. The campaign infrastructure is implemented. A qualified model
preset, a trained adapter and a 15-point improvement are **not established**.
Software regression results and model-quality results are separate gates.

## Frozen comparisons

The initial pilot uses the released beta.3 executable from commit
`fa0c8e4130ec4d13c59193436679c4e059c22dee`, SHA-256
`37a6e2eb0768411de9b51447c35f75e07b273a9da3b5ff83fbdae899abd76164`.
Its evaluator was copied from that commit before capability edits. Its four
declared cells are historical Python multi-file development and Rust feature
validation, once for each exact model. These previously inspected families are
not final holdouts. The pilot permits diagnosis but cannot establish a population
success rate or qualify an improvement. All four baseline attempts completed:

| Model | Historical task | Independent result | Turn seconds |
| --- | --- | --- | --- |
| MiMo 9B Heretic Q4 | Python multi-file | Passed | 291.80 |
| MiMo 9B Heretic Q4 | Rust feature | Failed: unexpected closing delimiter | 611.68 |
| Josiefied 7B Q4 | Python multi-file | Failed: missing `discounted` import | 610.88 |
| Josiefied 7B Q4 | Rust feature | Failed: unexpected closing delimiter | 613.58 |

The three failing turns reached cancellation under the 600-second model-turn cap;
cleanup and independent checking extend the measured wall time. These runs shared
the cloud CPU with compilation. Later candidate latency cannot be interpreted as
a causal speed improvement. The [full reports and raw text evidence](evidence/final-pass-2026-10-08/qualification/baseline/baseline-summary.json)
retain every failure, tool update, known/charged token count and sampled RSS.
Executable, SQLite, configuration and build artifacts are excluded from the
repository copy with an explicit excluded-file hash ledger. No candidate or final
holdout result is implied by this pilot.

The new [campaign manifest](final-pass-qualification-manifest.json) declares
34 disjoint family IDs: two development, two validation and 30 final holdouts.
Every family has three repetitions for each of two models and two arms, producing
408 planned slots, of which 360 are final. The
[accounting receipt](final-pass-qualification-accounting.json) includes every
unperformed slot; it does not remove failures from the denominator.

The exact Q4_K_M artifacts are:

| Model | Artifact SHA-256 |
| --- | --- |
| MiMo-V2.6-Distill-Qwen-9B-heretic | `00877ff79174b79f72c5ec704901fc958400a0b0d390fb61a068281618116dd1` |
| Josiefied-Qwen2.5-7B-Instruct-abliterated | `d7d626d96cc2d3567e8266101b4211b17634012dd99a0c9b28f475ce4eb620b6` |

The shared condition uses CPU inference, two threads, batch 128, 8,192 native
context, compact tools, OpenAI adapter, model-authored plans, 1,024 tokens per
response, 8,192 generated-token allowance, 12 requests, temperature 0.2, top-p
0.95 and a 600-second turn cap. The manifest hashes Goose, llama.cpp b11429,
the models, evaluator, controller and baseline. These CPU conditions do not
certify 8 GB Pascal VRAM, fit at larger context or latency on a GTX 1070.

Final tasks cover real faulty-code repair, new functions, repository navigation,
numeric boundaries, configuration, multi-file dependencies, actual TUI follow-up
corrections and pre-existing dirty Git files. The new holdouts are deliberately
small Python repositories; Rust and JavaScript remain in historical validation.
This is a bounded repository-task measure, not a general software-engineering or
unrestricted-ability benchmark. The [oracle self-check receipt](final-pass-qualification-oracles.json)
records all 30 broken-seed, golden, stale-source and early-exit checks. Reference
implementations are never included in model prompts.

## Run without contaminating the final comparison

1. Use development/validation outcomes to finish the candidate. Do not inspect
   final model outcomes, including the final baseline, during that work.
2. Create a new campaign directory and freeze the exact candidate binary and
   source identity. Candidate presets and embedded prompts are part of this
   treatment. Both arms use identical evaluator and effort conditions.
3. Execute final slots from the frozen controller. A started slot is never
   automatically retried, even after a crash. Retain that interruption and use a
   separately declared experiment if a rerun is necessary.
4. Publish the complete ledger and family-level paired intervals. Do not select
   prompts, training examples, adapters or favorable subsets using final results.

```bash
python3 scripts/qualification_campaign.py create \
  --output /path/to/new-campaign --baseline /path/to/verified-beta3-alt \
  --engine /path/to/goose --runtime /path/to/llama-server \
  --model-dir /path/to/exact-two-ggufs
python3 /path/to/new-campaign/evaluator/qualification_campaign.py seal \
  --root /path/to/new-campaign --candidate /path/to/candidate-alt \
  --source-commit EXACT_COMMIT
python3 /path/to/new-campaign/evaluator/qualification_campaign.py run \
  --root /path/to/new-campaign --partition final-holdout \
  --shard-index 0 --shard-count 32 --max-attempts 12
python3 /path/to/new-campaign/evaluator/qualification_campaign.py summary \
  --root /path/to/new-campaign --output /path/to/new-summary.json
```

Use `--source-dirty` when the candidate was built from a dirty checkout; its binary
hash remains authoritative. Keep the exact tool paths available. Shards keep both
arms of a family/model/repetition together and alternate arm order. Exported
capsules must retain executable permissions and all frozen evaluator files.

The opt-in [model workflow](../.github/workflows/model-matrix.yml) verifies the
published beta.3 checksum and source attestation, builds/tests the candidate,
freezes it before inspection, fetches only the two declared models, and uploads
each job's failures and raw evidence. A complete final run needs 32 shards with
12 attempts per shard; smaller budgets intentionally leave slots unperformed.
The workflow does not publish presets or push result commits. A partial capsule
can be resumed locally with its frozen controller; retain the original claimed
slots rather than dispatching a new run and combining selected successes.

Budget: 360 final attempts × 600 seconds is up to **60 inference-hours**, before
loading and independent checks. All 408 slots are up to 68 inference-hours.
The bounded outer watchdog is 960 seconds per attempt. Thirty-two concurrent
workers also multiply model download, memory and runner costs. The repository
does not present the unexecuted matrix as completed cloud work.

Each attempt records independent source behavior, native completion/stop reason,
failed tool updates, interventions, wall time, charged and known output tokens,
actual server prompt/generation timings when available and sampled process-tree
RSS. Missing token/VRAM observations remain missing. Unsupported completion
claims require review of retained prose and checks; an unreviewed claim is not
counted as accurate. The summary reports paired task-family bootstrap intervals
only after all final families and repetitions for a model are measured. Repeats
are not treated as independent families. The declared lift target is 15 points;
no script silently promotes a preset when a partial score looks favorable.

## Persistent workspace gate

`tests/long_lived_workspace.rs` performs 2,000 real Store session creations and
8,000 synthetic history events across five projects, 50 external edit/reindex
cycles and 50 cancelled index attempts. It archives 1,000 old sessions, restores
one history, exports 32 old synthetic inference payloads while protecting one
active lease, performs a state backup/restore, and checks SQLite integrity and
reopened history/project state. These events are explicitly fixtures, not model
conversations or novice testing.

The cloud run passed: one explicit ignored test executed in 8.01 seconds. Its
[receipt](evidence/final-pass-2026-10-08/qualification/longevity-receipt.json)
and [test log](evidence/final-pass-2026-10-08/qualification/longevity.log) retain
the actual counts and backup hashes. Package update/rollback is a separate gate
that consumes this persistent state.

```bash
ALT_LONGEVITY_ROOT=/path/to/new-persistent-longevity-run \
  cargo test --test long_lived_workspace -- --ignored --nocapture
```

Keep this state for the package upgrade/rollback gate; the test's receipt says
`installer_update_performed: false` until a separate installer test actually
uses it. The reference PC, accelerated inference, graphical terminals and five
uncoached novice sessions remain separate physical/human qualification.

## Training readiness

The [current readiness receipt](../training/status/final-pass-readiness.json)
records zero approved training/validation families, existing pending captures,
missing corpus/collection/loader inputs and no usable CUDA device. No training,
backward pass, adapter export or trained-model promotion occurred in this cloud.

The [training guide](../training/README.md#7-qualify-readiness-then-run-the-pilot)
now requires a complete collection ledger and at least 32 independently reviewed
training families plus eight validation families. Records must bind hashed actual
passing-attempt receipts to a separate predeclared slot plan, unchanged
trace/transcript/source/check evidence and correctness, privacy,
rights and split reviews. Failed, interrupted and unperformed attempts remain
in the collection ledger. Final families are excluded, including renaming them.

`qualify_loader.py` plans without loading weights by default. Explicit execution
uses the exact reviewed uncensored Safetensors parent and immutable revision,
the pinned package stack and a measured CUDA forward/backward pass. It saves no
adapter and takes no optimizer step. `readiness.py` binds those receipts to the
current GPU and prepared corpus before `train_sft.py --train` can proceed.
Q4 inference GGUFs cannot substitute for the training parent. Actual pilot fit,
resume, high-precision comparison, export and separate Q4 qualification remain
required; a successful training job alone would not establish stronger reasoning.
