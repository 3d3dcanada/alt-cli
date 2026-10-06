# Measure a selected model and runtime

Use **Settings → Qualify model contexts and restart** to run measurements or review
saved results. This keeps the selected model and settings unchanged. Each context
records three generation samples, runtime loading, sampled RAM/VRAM, the observed
runtime properties, cancellation after generated content, process cleanup, restart
and its health response. Errors and cancelled attempts remain in `evaluations/`.
The screen explains the measurements; the JSON file retains full details.

Only explicitly selected uncensored/abliterated profiles are eligible. For managed
models Alt verifies every GGUF part before starting it. Identity records include
artifact hash, executable and adjacent runtime library hashes, selected engine,
template where observable, native tool schemas, instructions, source build,
context, step budget, tool focus, access and runtime settings. An external tag
without verified weights or an observable template has incomplete identity and
must not be promoted to a supposedly equivalent tested configuration.

A starting CPU procedure:

```bash
alt --data-dir ./qualification-state models import /models/selected.gguf --uncensored --use
alt --data-dir ./qualification-state runtime --gpu-layers 0 --threads 2 --batch 128
alt --data-dir ./qualification-state qualify --contexts 2048,4096,8192 --repeats 3
```

Choose the runtime executable through Settings when it is not auto-detected.
The requested context and observed native context are separate fields. For external
OpenAI-compatible servers, set the native context in that application first; an
API request or Alt setting alone does not prove the server allocated that window.
External processes are never stopped by Alt qualification. Their stop/restart and
resource measurements require an operator-side run and remain explicitly unmeasured.

The optional **Managed runtime settings → Reasoning mode** choice passes
`enable_thinking` to the selected model's chat template. The default makes no
override. Only a template supporting that variable honors it. CLI equivalents are
`alt runtime --thinking false`, `--thinking true`, and `--thinking-default`.
Identity records include the choice. Reaching an output limit is a reason to test
this option, not evidence that disabling reasoning improves accuracy. Retain both
successful and failed task attempts; no preset is selected automatically.

## Physical qualification still required

| Target | Procedure | Required evidence |
|---|---|---|
| GTX 1070, 8 GB VRAM, Linux, 16 GB RAM | Use a Pascal `sm_61` compatible CUDA 12 runtime; inspect `--list-devices`; select conservative GPU layers in Settings; run contexts 2K/4K/8K three times, then independent coding tasks | Driver/runtime/library/model hashes, actual device, peak RAM and VRAM, load failures, generated-token stop/restart, every task attempt |
| Newer NVIDIA GPU | Repeat the same artifact and tasks with that device's supported runtime; vary one context/layer/cache choice at a time | Separate compatibility row for each actual configuration; no extrapolation from GTX or CPU |
| Older CPU-only computer | Start with small Q4 weights, 2K/4K context, batch 128 or lower; run the same matrix | Actual physical RAM/CPU and latency, not only a container memory limit |
| LM Studio / ORA / compatible endpoint | Select the exact loaded uncensored model, test inventory, run `doctor`, `evaluate`, `benchmark` and `qualify`; record application version, model hash, template and native context in the server | Native tool roundtrip, project oracles, authentication behavior and operator-side stop/restart; no inferred runtime identity from an OpenAI URL |
| Private/gated Hugging Face repository | Set your existing authorized `HF_TOKEN` in your environment; search/download from Models and verify every selected part; repeat interruption/resume and expired-token recovery | A real authorized account/repository; redact credentials and keep origin-scoping tests separate from an authenticated success |

Do not use a CUDA 13-only build for Pascal. The managed runtime is CPU first and
has a different glibc requirement from the Alt frontend; an old-distribution
frontend launch does not prove that every downloadable runtime works there.
Minimal Linux installations also need the runtime's shared libraries. For the
pinned CPU archive, Debian/Ubuntu's `libgomp1` provides `libgomp.so.1`; some custom
builds also need `libnuma1`. Alt reports a missing library with the package hint
and keeps the selected model unchanged. It does not install system packages
silently. The retained minimal-Ubuntu failure is separate from successful runs
in a Debian 12 container with the required library already installed.

Constrained-memory tests should declare their actual Linux limit, for example a
Docker run with `--memory 512m --memory-swap 512m --cpus 2`. A failed memory preflight
is expected for multi-gigabyte weights at that limit. Retain the failure and verify
no substitute model was loaded. Such tests exercise failure handling and do not
stand in for a physical GTX 1070 or a real 16 GB machine.

Generation throughput is not coding reliability. Capability probes measure only
their named task. Use the independent project suite before recommending a preset.

The repeatable container check is:

```bash
python3 scripts/qualify_memory_limit.py --binary target/portable-glibc231/release/alt \
  --runtime /path/to/llama-server --model /path/to/uncensored.gguf \
  --sha256 VERIFIED_SHA256 --uncensored --output /path/to/new-memory-report
```

It verifies the exact artifact, records the actual cgroup budget, expects a RAM
preflight failure at 512 MiB, and checks that the selected profile is unchanged.
