# 0.7 owner-PC handoff: A08 (CUDA 12 runtime) and A10 (matched re-run)

Prepared in the cloud with fixtures only; nothing here has been run on a GPU.
Reference PC: Linux, GTX 1070 8 GB (compute 6.1), 16 GB RAM.

## Before you start

```bash
git checkout cloud/0.7-2026-10-11      # or main after the PR merges
cargo build --locked --release
bash scripts/setup-cloud.sh             # optional full gate run; needs /workspace
```

Free the GPU first (close other GPU programs; `ollama stop <tag>`). Every step
below keeps its raw output under the folder you choose; record failures too.

## A08: verify and pin a CUDA 12 runtime

1. Build: `scripts/build-pascal-runtime.sh ~/llama-cuda12` (CUDA 12.x toolkit,
   driver 525.60+; CUDA 13 cannot target sm_61).
2. Check it on this PC with a model you already have:
   `ALT_BINARY=target/release/alt scripts/pc_cuda_check.sh ~/llama-cuda12/build/bin/llama-server /path/to/MiMo-9B-Q4_K_M.gguf ~/alt-cuda-check`
   Expect `--list-devices` to show `CUDA0: … GTX 1070`, the benchmark to finish,
   and about 6.3 GB VRAM used at 16K with q8 KV (the 2026-10-08 trial value).
3. Pin it for automatic installation (optional):
   `scripts/pin_cuda_runtime.py ~/llama-cuda12/build/bin ~/alt-pins` prints a
   deterministic archive, its SHA256/size and the exact `pub const CUDA12` value.
   Check the licence notes it prints (llama.cpp is MIT; bundling `libcudart` /
   `libcublas` needs `--bundle-cuda-libs` and the NVIDIA CUDA EULA's
   redistribution terms). Upload the archive as an alt-cli release asset, replace
   `None` in `src/runtime.rs` with the printed value and its real URL, run
   `cargo test --test cuda_runtime`, and open a PR.
4. In Alt: Settings → Runtime → **GPU runtime (CUDA 12)**. CPU stays the default;
   installing never selects the CUDA runtime or changes GPU layers.

## A10: matched re-run of the 2026-10-08 trial

`scripts/pc_matrix_0_7.sh` runs suite v5 (8 cases), 16K context, compact tools,
4,096 output tokens, 3 repeats, two arms (`trial` = the trial's settings,
`thinking-off` = A01) for:

| Cell | Where | Label |
| --- | --- | --- |
| Spark-X2.5 4B Q8 abliterated | Ollama, GPU | uncensored |
| MiMo V2.6 Distill 9B Q4_K_M abliterated | llama.cpp CUDA 12 (managed by Alt) | uncensored |
| MiMo V2.6 Distill 9B Q4_K_M standard | llama.cpp CUDA 12 (managed by Alt) | standard |
| moonshotai/kimi-k3, z-ai/glm-5.3 | NVIDIA NIM | hosted |

```bash
bash scripts/pc_matrix_0_7.sh --dry-run          # prints and validates every cell; runs nothing
export GOOSE=~/.local/share/alt-cli/tools/goose ALT_BIN=$PWD/target/release/alt
export LLAMA_CUDA=~/llama-cuda12/build/bin/llama-server
export MIMO_ABL=/models/mimo-abliterated.gguf MIMO_ABL_SHA=<sha256>
export MIMO_STD=/models/mimo-standard.gguf    MIMO_STD_SHA=<sha256>
export SPARK_TAG=<your ollama tag> NVIDIA_API_KEY=<from build.nvidia.com>
bash scripts/pc_matrix_0_7.sh                    # results in ./pc-matrix-0.7-<date>/
```

Before each GPU cell the script checks free VRAM (6,800 MiB for MiMo, 5,200 MiB
for Spark; change with `MIMO_NEED_MIB`/`SPARK_NEED_MIB`) and records a skip
instead of starting. It unloads Spark from Ollama between cells and stops any
llama-server it started. `ledger.jsonl` records every cell, including skips and
non-zero exits. Labels are recorded as given (`--artifact-label`); hosted runs
need no local file hash.

Report: compare per-model pass counts with the trial table in
`docs/research/2026-10-08-pc-model-trial/README.md`, keep failure classes
(budget / wrong / timeout / provider fault / thinking cutoff), and treat
one-task differences as noise. Raw run folders hold full request logs; keep
them off git and publish summaries (`scripts/check_repo_weight.py` enforces 1 MiB).
