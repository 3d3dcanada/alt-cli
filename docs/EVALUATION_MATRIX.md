# Reproducible coding-task matrices

The 0.6 GitHub workflow runs an entire 120-attempt matrix for an explicitly
selected pinned Spark or Qwen uncensored checkpoint. Dispatch **Complete
uncensored model matrix** and choose the model. Each campaign builds one frozen
executable, then runs eight disjoint shards: 100 development attempts and 20
held-out attempts at 8K context, five repetitions per task. A green measurement
job means all assigned attempts were retained, not that the model solved them.

`scripts/summarize_matrix.py` checks every expected cell, raw evidence hashes,
unchanged oracles and matching executable/model/runtime/settings across shards.
`scripts/compare_matrices.py` rejects comparisons with changed model, harness,
runtime or budget controls. Generation seeds and underlying hosted CPU hardware
are not matched; comparisons are descriptive counts, not statistical proof.
The workflow preserves failures and timeouts in a dedicated results branch as
well as Actions artifacts. It never silently switches models or promotes a preset.

`scripts/measure_live_memory.py` separately probes saved decisions, 200 distracting
notes, process restarts and a source edit. It uses the same selected uncensored
model for all three observations. The request contains no expected answer values.
This narrow recall probe is separate from coding-task success and cannot enlarge
the model's native context. See the current [implementation record](IMPLEMENTATION_0.6.md).

`acceptance_projects.py` defines 20 development tasks and four held-out tasks.
They cover Python/Rust/JavaScript features, multi-file repairs, dependency imports,
CLI setup, configuration, CSV precision, SQLite migration/transactions, paths,
caching, retries, concurrency, JSONL, module-relative setup, service configuration,
and a local HTTP/SQL-injection repair. Held-out tasks cover stable deduplication,
atomic writes, dependency ordering and timezone conversion.

Run `python3 scripts/acceptance_projects.py` first. Every oracle must reject its
broken seed and accept a known repair. Additional negative controls reject selected
incomplete repairs. This validates the fixture, not a model's ability to solve it.
Do not tune prompts on held-out task results and then describe those tasks as unseen.

Oracle contract 4 requires the assertion runner to emit a fresh completion receipt
only after the checks finish. Exit 0 alone is insufficient: Python, Rust and
JavaScript early-exit negative controls exercise that failure. The Rust oracle
compiles against the actual project snapshot and verifies that its named test
ran. Collection copies only declared assertion inputs, never generated compiler
output. `scripts/smoke_acceptance_collection.py` exercises collection, snapshot
behavior and structured verification through actual Alt processes. Earlier
contract-3 cohorts remain historical and cannot be compared as matched baselines.
These checks establish the recorded fixture behavior; Full access is not a
sandbox for adversarial oracle code.

```bash
python3 scripts/live_acceptance.py \
  --binary target/portable-glibc231/release/alt --engine /path/to/goose \
  --runtime /path/to/llama-server --model /path/to/uncensored.gguf \
  --sha256 VERIFIED_SHA256 --uncensored --contexts 4096,8192 \
  --partition development --repeats 5 --threads 2 --timeout 240 \
  --tool-profile all --output /path/to/new-campaign
```

A complete single-context development configuration has 100 attempts. Run held-out
as a separate campaign (20 attempts per context). `--stop-after N` intentionally
creates an incomplete pilot; it cannot satisfy the full acceptance gate. `--resume`
requires the same binary, weights, fixtures, engine/runtime hashes, task set and
settings. It verifies saved evidence hashes before skipping completed attempts.
Incomplete directories are retained under `interrupted/` before a clean retry.

Each attempt retains original and resulting source, external oracles and hashes,
setup output, raw turn JSONL, stderr, independent before/after results, elapsed
time, process outcome, sampled descendant memory, and context views. The campaign
freezes the binary and harness sources. External inference-server memory is not
included in Alt's descendant RSS. Use a separate qualification report for runtime
and hardware measurements. CPU sharing and build activity must be disclosed when
interpreting throughput.

`--tool-profile` selects `all`, `inspect`, `coding` or `terminal` schemas. These are
explicit user choices and do not change access policy. Coding focus needs named
checks if the task requires execution; All or Terminal exposes arbitrary shell
commands when Full access is selected. A smaller tool menu is a hypothesis to test,
not a blanket claim of higher capability.

`compare_acceptance.py BASELINE CANDIDATE --output comparison.json` rejects mismatched
weights, fixtures, runtime/engine, contexts, repeats, threads and time budgets. It
reports source success, protocol turn endings and Wilson intervals separately.
An `end_turn` stop reason is distinct from `max_tokens`, but it is still only a
protocol outcome. Goose can return it with a message that the action budget was
reached. It does not establish that the requested behavior works; derived reports
identify those budget notices separately. The report
marks incomplete campaigns. Final-prose claim accuracy remains unassessed until a
reviewer checks the retained prose against the actual source and tool evidence.
Success is never inferred from an exit code or fluent explanation.

For a managed runtime whose template supports it, `--thinking off` or `on` records
and applies an explicit reasoning override. Default leaves the template unchanged.
This switch is rejected for external servers, where the operator must configure
and record it separately. A changed reasoning mode is a different configuration
and cannot be pooled into a matched baseline comparison.

Preset promotion requires complete development and held-out matrices on the exact
configuration, reviewed false claims, acceptable task latency and applicable
physical hardware measurements. A native tool probe, synthetic protocol fixture,
small pilot or successful throughput benchmark alone cannot promote a preset.

After validating the full matrix:

```bash
python3 scripts/analyze_matrix.py RETAINED_MATRIX --model spark \
  --claims MANUAL_REVIEWS.json --output analysis.json
```

The source directory must contain `summary.json` and all eight shard directories.
Manual rows must match the model, case, repeat and original report SHA-256. The
analysis records reviewed/unreviewed counts and separates action-budget messages
from protocol stop reasons and behavioral success. A claim classification is a
reviewer's primary assessment, not automatic proof that every sentence is true.
