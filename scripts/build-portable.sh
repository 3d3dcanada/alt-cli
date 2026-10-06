#!/usr/bin/env bash
set -euo pipefail
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
export CARGO_HOME=/workspace/.cargo
export RUSTUP_HOME=/workspace/.rustup
export PATH="/workspace/.cargo/bin:$PATH"
export DOCKER_CONFIG="${DOCKER_CONFIG:-/workspace/.alt-tools/docker}"
mkdir -p "$DOCKER_CONFIG"
# Docker is opt-in for producing the release artifact; never required to chat.
# All compiled outputs/caches stay in the existing workspace.
docker build -t alt-build-glibc231 "$repo_dir/scripts/portable"
docker run --rm --user "$(id -u):$(id -g)" \
  -v /workspace:/workspace -v "$repo_dir:$repo_dir" -w "$repo_dir" \
  -e CARGO_HOME=/workspace/.cargo -e RUSTUP_HOME=/workspace/.rustup \
  -e PATH=/workspace/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
  alt-build-glibc231 cargo build --release --locked --target-dir target/portable-glibc231
python3 "$repo_dir/scripts/package-linux.py" --binary "$repo_dir/target/portable-glibc231/release/alt"
