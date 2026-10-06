# Reproducible coding-task matrices

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
reports source success, naturally ended turns and Wilson intervals separately.
An `end_turn` stop reason is distinct from a transport response or `max_tokens`;
even a natural end does not establish that the requested behavior works. The report
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
