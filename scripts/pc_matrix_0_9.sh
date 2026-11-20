#!/usr/bin/env bash
# 0.9 owner-PC matrix: suite v5 (8 cases), 16K context, compact tools, 4,096 output,
# Guided access (no terminal), one repeat, 300 s per case, two arms (trial,
# thinking-off). Differs from the 0.7 matrix (pc_matrix_0_7.sh, kept as the record
# of that plan): Guided instead of Trusted + terminal, a 300 s limit, Spark 4B on
# the owned llama.cpp runtime instead of Ollama, current hosted model IDs.
#
# Serial on purpose: one case at a time, local GPU cells first, then hosted cells
# in the order given (slowest last), so a deadline cuts the least informative
# cells. Every local case passes a GPU headroom gate first and every llama-server
# it started is stopped by PID. One harness call per case: a crash costs one case,
# and a re-run resumes (finished cases are skipped, interrupted ones resumed).
#
# Environment (paths absolute):
#   ALT_BIN (target/release/alt)  GOOSE (required)  LLAMA_CUDA (CUDA 12 llama-server, local cells)
#   MODELS          folder holding the GGUF files named in LOCAL_CELLS
#   NVIDIA_MODELS   hosted ids, in run order
#   LOCAL_CELLS     "name:label:relative/file.gguf:need_mib ..."
#   CASES (all eight v5 cases)  ARMS (trial thinking-off)  TIMEOUT (300)  CONTEXT (16384)  OUTPUT (4096)
#   DEADLINE_HOURS (8)  GATE (ora-can-i-start style command: GATE --need-mb N; empty = nvidia-smi free)
#   OUT             results folder (default ./pc-matrix-0.9-<date>)
# NVIDIA_API_KEY must be set for hosted cells; only its name is passed to Alt.
set -euo pipefail
dry=false
[[ "${1:-}" == "--dry-run" ]] && dry=true
repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
ALT_BIN="${ALT_BIN:-$repo/target/release/alt}"
MODELS="${MODELS:-}"
LOCAL_CELLS="${LOCAL_CELLS:-mimo-9b-abl:uncensored:mimo-v26-abliterated/MiMo-V2.6-Distill-Qwen-9B-Abliterated-Q4_K_M.gguf:6800 mimo-9b-std:standard:mimo-v26-distill-9b/MiMo-V2.6-Distill-Qwen-9B-Q4_K_M.gguf:6800 qwen35-9b:standard:qwen35-9b-gguf/Qwen3.5-9B-Q4_K_M.gguf:6800 spark-4b:uncensored:spark-x2.5-4b/Spark-X2.5-4B-Q8_0.gguf:5600}"
NVIDIA_MODELS="${NVIDIA_MODELS:-minimaxai/minimax-m3.1-flash moonshotai/kimi-k3.1-pro z-ai/glm-5.3-flash}"
ARMS="${ARMS:-trial thinking-off}"
TIMEOUT="${TIMEOUT:-300}" CONTEXT="${CONTEXT:-16384}" OUTPUT="${OUTPUT:-4096}" DEADLINE_HOURS="${DEADLINE_HOURS:-8}"
GATE="${GATE:-}"
OUT="${OUT:-$PWD/pc-matrix-0.9-$(date +%Y-%m-%d)}"
CASES="${CASES:-python-feature python-multifile rust-feature javascript-config python-http-security heldout-toposort sealed-intervals sealed-cache-expiry}"
ledger="$OUT/ledger.jsonl"
deadline=$(( $(date +%s) + DEADLINE_HOURS * 3600 ))

log() { printf '%s\n' "$*"; }
record() { $dry || printf '%s\n' "$1" >> "$ledger"; }

# Wait up to 10 minutes for GPU headroom; a refusal is recorded, never forced.
gpu_ready() {
  local need="$1" tries=0
  $dry && { log "  [dry-run] would require ${need} MiB free VRAM"; return 0; }
  while true; do
    if [[ -n "$GATE" ]]; then $GATE --need-mb "$need" >/dev/null 2>&1 && return 0
    else
      local free; free="$(nvidia-smi --query-gpu=memory.free --format=csv,noheader,nounits 2>/dev/null | sort -n | tail -1 || true)"
      [[ -n "$free" && "$free" -ge "$need" ]] && return 0
    fi
    (( ++tries > 20 )) && return 1
    sleep 30
  done
}

servers_before=""
remember_servers() { servers_before="$(pgrep -x llama-server || true)"; }
stop_new_servers() {
  local pid
  for pid in $(pgrep -x llama-server || true); do
    grep -qx "$pid" <<<"$servers_before" || { log "  stopping leftover llama-server $pid"; kill "$pid" 2>/dev/null || true; }
  done
}
trap 'stop_new_servers' EXIT

run_case() {  # cell arm case need_mib -- harness args...
  local cell="$1" arm="$2" case="$3" need="$4"; shift 5
  local dir="$OUT/$cell/$arm/$case" extra=()
  [[ "$arm" == "thinking-off" ]] && extra+=(--thinking-level off)
  local cmd=(python3 "$repo/scripts/live_acceptance.py" --suite v5 --binary "$ALT_BIN" --engine "${GOOSE:-GOOSE_UNSET}"
    --access guided --contexts "$CONTEXT" --repeats 1 --cases "$case" --tool-profile compact
    --output-tokens "$OUTPUT" --timeout "$TIMEOUT" --verification-plan --output "$dir" "$@" "${extra[@]}")
  if $dry; then
    log "  [dry-run] ${cmd[*]}"
    "${cmd[@]}" --check-args > /dev/null || { log "  INVALID arguments for $cell/$arm/$case"; exit 3; }
    return 0
  fi
  if [[ -f "$dir/summary.json" ]] && python3 -c 'import json,sys; s=json.load(open(sys.argv[1])); sys.exit(0 if s["total"]>=1 else 1)' "$dir/summary.json"; then
    log "  $cell/$arm/$case already measured"; return 0
  fi
  if (( $(date +%s) >= deadline )); then
    record "{\"cell\":\"$cell\",\"arm\":\"$arm\",\"case\":\"$case\",\"status\":\"not_reached\",\"reason\":\"${DEADLINE_HOURS} h deadline\"}"
    log "  $cell/$arm/$case not reached (deadline)"; return 0
  fi
  if [[ "$need" -gt 0 ]] && ! gpu_ready "$need"; then
    record "{\"cell\":\"$cell\",\"arm\":\"$arm\",\"case\":\"$case\",\"status\":\"skipped\",\"reason\":\"GPU headroom below ${need} MiB for 10 minutes\"}"
    log "  SKIP $cell/$arm/$case: GPU headroom"; return 0
  fi
  local resume=() status=0 started; started=$(date +%s)
  [[ -d "$dir" ]] && resume=(--resume)
  mkdir -p "$(dirname "$dir")"
  remember_servers
  "${cmd[@]}" "${resume[@]}" > "$dir.stdout" 2> "$dir.stderr" || status=$?
  stop_new_servers
  record "{\"cell\":\"$cell\",\"arm\":\"$arm\",\"case\":\"$case\",\"exit\":$status,\"wall_s\":$(( $(date +%s) - started )),\"load\":\"$(cut -d' ' -f1 /proc/loadavg)\",\"output\":\"$dir\"}"
  log "  $cell/$arm/$case exit $status"
}

log "Alt 0.9 PC matrix -> $OUT (dry-run: $dry)"
if ! $dry; then
  mkdir -p "$OUT"
  [[ -x "$ALT_BIN" && -x "${GOOSE:-}" ]] || { log "Set ALT_BIN and GOOSE to executables"; exit 2; }
  "$ALT_BIN" build-info > "$OUT/BUILD.json"
  nvidia-smi --query-gpu=name,memory.total,compute_cap,driver_version --format=csv > "$OUT/gpu.csv" || true
fi

for spec in $LOCAL_CELLS; do
  IFS=: read -r cell label file need <<<"$spec"
  log "- $cell (llama.cpp CUDA, $label)"
  path="$MODELS/$file"
  if ! $dry && [[ -z "${LLAMA_CUDA:-}" || ! -f "$path" ]]; then
    record "{\"cell\":\"$cell\",\"status\":\"skipped\",\"reason\":\"LLAMA_CUDA unset or model file missing\"}"
    log "  SKIP: LLAMA_CUDA unset or $path missing"; continue
  fi
  sha="SHA_DRY_RUN"
  if ! $dry; then
    sha_file="$OUT/$cell.sha256"
    [[ -s "$sha_file" ]] || sha256sum "$path" | cut -d' ' -f1 > "$sha_file"
    sha="$(cat "$sha_file")"
  fi
  for arm in $ARMS; do for case in $CASES; do
    run_case "$cell" "$arm" "$case" "$need" -- --artifact-label "$label" --runtime "${LLAMA_CUDA:-LLAMA_CUDA_UNSET}" \
      --model "$path" --sha256 "$sha" --gpu-layers 99 --cache-k q8_0 --cache-v q8_0 --flash-attention true
  done; done
done
for model in $NVIDIA_MODELS; do
  cell="nvidia-${model//\//-}"
  log "- $cell (NVIDIA NIM, hosted)"
  if [[ -z "${NVIDIA_API_KEY:-}" ]] && ! $dry; then
    record "{\"cell\":\"$cell\",\"status\":\"skipped\",\"reason\":\"NVIDIA_API_KEY not set\"}"
    log "  SKIP: NVIDIA_API_KEY not set"; continue
  fi
  for arm in $ARMS; do for case in $CASES; do
    run_case "$cell" "$arm" "$case" 0 -- --artifact-label hosted --provider openai --endpoint https://integrate.api.nvidia.com/v1 \
      --model-id "$model" --api-key-env NVIDIA_API_KEY --dialect chat-template
  done; done
done
log "Done. Ledger: $ledger. Record failures as well as passes."
