#!/usr/bin/env bash
# A10: matched 0.7 re-run of the 2026-10-08 PC trial (owner PC executes; the cloud
# only checks it with --dry-run). Same matrix as the trial note: suite v5, 8 cases,
# 16K context, compact tools, 4,096 output tokens, now 3 repeats, for
#   Spark-X2.5 4B Q8 abliterated (Ollama, GPU)            label uncensored
#   MiMo V2.6 Distill 9B Q4_K_M abliterated (llama.cpp CUDA)  label uncensored
#   MiMo V2.6 Distill 9B Q4_K_M standard (llama.cpp CUDA)     label standard
#   moonshotai/kimi-k3 and z-ai/glm-5.3 (NVIDIA NIM)        label hosted
# Arms: "trial" repeats the trial settings; "thinking-off" adds A01's thinking off.
# Every cell checks GPU headroom first, records skips, and stops every
# llama-server it started. Failed cells are kept; nothing is retried silently.
#
# Configure with environment variables (all paths absolute):
#   ALT_BIN       Alt binary under test            (default target/release/alt)
#   GOOSE         Goose 1.53.0 binary              (required)
#   LLAMA_CUDA    CUDA 12 llama-server             (MiMo cells)
#   MIMO_ABL MIMO_ABL_SHA   abliterated GGUF and SHA256
#   MIMO_STD MIMO_STD_SHA   standard GGUF and SHA256
#   SPARK_TAG     Ollama tag for Spark 4B          (default spark-x2.5:4b-q8)
#   OLLAMA        Ollama root                      (default http://127.0.0.1:11434)
#   NVIDIA_MODELS space-separated ids              (default "moonshotai/kimi-k3 z-ai/glm-5.3")
#   ARMS          space-separated arms             (default "trial thinking-off")
#   REPEATS (3)  CONTEXT (16384)  OUTPUT (4096)  TIMEOUT (600)
#   MIMO_NEED_MIB (6800)  SPARK_NEED_MIB (5200)  free VRAM required before a GPU cell
#   OUT           results folder                   (default ./pc-matrix-0.7-<date>)
# NVIDIA_API_KEY must be set for hosted cells; only its name is passed to Alt.
set -euo pipefail
dry=false
[[ "${1:-}" == "--dry-run" ]] && dry=true
repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
ALT_BIN="${ALT_BIN:-$repo/target/release/alt}"
SPARK_TAG="${SPARK_TAG:-spark-x2.5:4b-q8}"
OLLAMA="${OLLAMA:-http://127.0.0.1:11434}"
NVIDIA_MODELS="${NVIDIA_MODELS:-moonshotai/kimi-k3 z-ai/glm-5.3}"
ARMS="${ARMS:-trial thinking-off}"
REPEATS="${REPEATS:-3}" CONTEXT="${CONTEXT:-16384}" OUTPUT="${OUTPUT:-4096}" TIMEOUT="${TIMEOUT:-600}"
MIMO_NEED_MIB="${MIMO_NEED_MIB:-6800}" SPARK_NEED_MIB="${SPARK_NEED_MIB:-5200}"
OUT="${OUT:-$PWD/pc-matrix-0.7-$(date +%Y-%m-%d)}"
CASES="python-feature,python-multifile,rust-feature,javascript-config,python-http-security,heldout-toposort,sealed-intervals,sealed-cache-expiry"
ledger="$OUT/ledger.jsonl"

log() { printf '%s\n' "$*"; }
record() { $dry || printf '%s\n' "$1" >> "$ledger"; }

free_vram_mib() {
  nvidia-smi --query-gpu=memory.free --format=csv,noheader,nounits 2>/dev/null | sort -n | tail -1
}

# "ora-can-i-start": enough free VRAM for this cell, or skip and record why.
can_start() {
  local need="$1" cell="$2"
  if $dry; then log "  [dry-run] would require ${need} MiB free VRAM for $cell"; return 0; fi
  local free; free="$(free_vram_mib || true)"
  if [[ -z "$free" || "$free" -lt "$need" ]]; then
    log "  SKIP $cell: free VRAM ${free:-unknown} MiB < ${need} MiB"
    record "{\"cell\":\"$cell\",\"status\":\"skipped\",\"reason\":\"free VRAM ${free:-unknown} MiB below ${need} MiB\"}"
    return 1
  fi
}

servers_before=""
remember_servers() { servers_before="$(pgrep -x llama-server || true)"; }
stop_new_servers() {
  local pid
  for pid in $(pgrep -x llama-server || true); do
    grep -qx "$pid" <<<"$servers_before" || { log "  stopping leftover llama-server $pid"; kill "$pid" 2>/dev/null || true; }
  done
}
unload_ollama() {
  $dry && { log "  [dry-run] would unload $1 from Ollama (keep_alive 0)"; return; }
  curl -fsS "$OLLAMA/api/generate" -d "{\"model\":\"$1\",\"keep_alive\":0}" >/dev/null 2>&1 || true
}
trap 'stop_new_servers' EXIT

run_cell() {
  local cell="$1"; shift
  local arm="$1"; shift
  local extra=()
  [[ "$arm" == "thinking-off" ]] && extra+=(--thinking-level off)
  local cmd=(python3 "$repo/scripts/live_acceptance.py" --suite v5 --binary "$ALT_BIN" --engine "${GOOSE:-GOOSE_UNSET}"
    --contexts "$CONTEXT" --repeats "$REPEATS" --cases "$CASES" --tool-profile compact
    --output-tokens "$OUTPUT" --timeout "$TIMEOUT" --verification-plan
    --output "$OUT/$cell-$arm" "$@" "${extra[@]}")
  if $dry; then
    log "  [dry-run] ${cmd[*]}"
    "${cmd[@]}" --check-args > /dev/null || { log "  INVALID arguments for $cell/$arm"; exit 3; }
    return 0
  fi
  remember_servers
  local status=0
  "${cmd[@]}" > "$OUT/$cell-$arm.stdout" 2> "$OUT/$cell-$arm.stderr" || status=$?
  stop_new_servers
  record "{\"cell\":\"$cell\",\"arm\":\"$arm\",\"exit\":$status,\"output\":\"$OUT/$cell-$arm\"}"
  log "  $cell/$arm exit $status"
}

log "Alt 0.7 PC matrix → $OUT (dry-run: $dry)"
if ! $dry; then
  mkdir -p "$OUT"
  [[ -x "$ALT_BIN" && -x "${GOOSE:-}" ]] || { log "Set ALT_BIN and GOOSE to executables"; exit 2; }
  "$ALT_BIN" build-info > "$OUT/BUILD.json"
  nvidia-smi --query-gpu=name,memory.total,compute_cap,driver_version --format=csv > "$OUT/gpu.csv" || true
fi

for arm in $ARMS; do
  log "== arm $arm"
  log "- spark-4b (Ollama $SPARK_TAG)"
  if can_start "$SPARK_NEED_MIB" "spark-4b-$arm"; then
    run_cell spark-4b "$arm" --artifact-label uncensored --provider ollama --endpoint "$OLLAMA" --model-id "$SPARK_TAG"
  fi
  unload_ollama "$SPARK_TAG"
  for variant in abl std; do
    if [[ "$variant" == abl ]]; then file="${MIMO_ABL:-}"; sha="${MIMO_ABL_SHA:-}"; label=uncensored
    else file="${MIMO_STD:-}"; sha="${MIMO_STD_SHA:-}"; label=standard; fi
    log "- mimo-9b-$variant (llama.cpp CUDA, $label)"
    if [[ -z "$file" || -z "${LLAMA_CUDA:-}" ]] && ! $dry; then
      record "{\"cell\":\"mimo-9b-$variant\",\"arm\":\"$arm\",\"status\":\"skipped\",\"reason\":\"MIMO_${variant^^} or LLAMA_CUDA not set\"}"
      log "  SKIP: set MIMO_${variant^^}, MIMO_${variant^^}_SHA and LLAMA_CUDA"; continue
    fi
    if can_start "$MIMO_NEED_MIB" "mimo-9b-$variant-$arm"; then
      run_cell "mimo-9b-$variant" "$arm" --artifact-label "$label" --runtime "${LLAMA_CUDA:-LLAMA_CUDA_UNSET}" \
        --model "${file:-MIMO_${variant^^}_UNSET}" --sha256 "${sha:-SHA_UNSET}"
    fi
  done
  for model in $NVIDIA_MODELS; do
    cell="nvidia-${model//\//-}"
    log "- $cell (NVIDIA NIM, hosted)"
    if [[ -z "${NVIDIA_API_KEY:-}" ]] && ! $dry; then
      record "{\"cell\":\"$cell\",\"arm\":\"$arm\",\"status\":\"skipped\",\"reason\":\"NVIDIA_API_KEY not set\"}"
      log "  SKIP: NVIDIA_API_KEY not set"; continue
    fi
    run_cell "$cell" "$arm" --artifact-label hosted --provider openai --endpoint https://integrate.api.nvidia.com/v1 \
      --model-id "$model" --api-key-env NVIDIA_API_KEY --dialect chat-template
  done
done
log "Done. Ledger: $ledger. Summarise with scripts/summarize_matrix.py; record failures as well as passes."
