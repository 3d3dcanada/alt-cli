#!/usr/bin/env bash
# A08 (owner PC, not run in the cloud): check that a CUDA 12 runtime serves the
# GPU before pinning it. Read-only apart from Alt's state folder you pass in.
# Usage: scripts/pc_cuda_check.sh /path/to/llama-server /path/to/model.gguf [STATE_DIR]
set -euo pipefail
server="${1:?llama-server path}"; model="${2:?GGUF model path}"; state="${3:-$(mktemp -d)}"
alt="${ALT_BINARY:-target/release/alt}"
echo "== GPU"; nvidia-smi --query-gpu=name,memory.total,compute_cap,driver_version --format=csv,noheader
echo "== runtime devices"; "$server" --list-devices
echo "== Alt view"; "$alt" --data-dir "$state" hardware | head -40
"$alt" --data-dir "$state" models import "$model" --uncensored --use
"$alt" --data-dir "$state" runtime --gpu-layers 99 --cache-k q8_0 --cache-v q8_0 --flash-attention true
python3 - "$state" "$server" <<'PY'
import sys, tomllib, pathlib
state, server = map(pathlib.Path, sys.argv[1:])
prefs = state / "preferences.toml"
text = prefs.read_text()
text = "\n".join(l for l in text.splitlines() if not l.startswith("runtime_path"))
prefs.write_text(f'runtime_path = "{server}"\n' + text + "\n")
PY
echo "== benchmark (16K context recommended for the 0.7 matrix)"
"$alt" --data-dir "$state" benchmark
nvidia-smi --query-gpu=memory.used --format=csv,noheader
echo "State kept in $state; stop any leftover llama-server with: pkill -f \"$server\""
