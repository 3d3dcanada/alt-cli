#!/usr/bin/env bash
set -euo pipefail
# User-invoked source build for NVIDIA Pascal (GTX 1070: compute capability 6.1).
# CUDA toolkit 12.x is required; no packages/drivers are installed by this script.
runtime_destination="${1:?Pass an empty build destination}"
command -v nvcc >/dev/null || { echo 'Install a compatible CUDA 12.x toolkit first.' >&2; exit 1; }
nvcc --version | grep -q 'release 12\.' || { echo 'Use CUDA 12.x; CUDA 13 drops offline compilation for Pascal.' >&2; exit 1; }
if [[ -e "$runtime_destination" ]]; then echo 'Choose a new directory; existing work is preserved.' >&2; exit 1; fi
git clone --filter=blob:none --no-checkout https://github.com/ggml-org/llama.cpp.git "$runtime_destination"
git -C "$runtime_destination" checkout --detach d81235049384534c167caea52b85a694f6103d14
cmake -S "$runtime_destination" -B "$runtime_destination/build" -DCMAKE_BUILD_TYPE=Release -DGGML_CUDA=ON -DCMAKE_CUDA_ARCHITECTURES=61 -DGGML_NATIVE=OFF
cmake --build "$runtime_destination/build" --config Release --target llama-server -j "${ALT_BUILD_JOBS:-2}"
"$runtime_destination/build/bin/llama-server" --list-devices
printf 'Select this runtime in Alt Settings: %s/build/bin/llama-server\nStart with a 4K context, batch 128 and conservative GPU layers; benchmark before increasing them.\n' "$runtime_destination"
