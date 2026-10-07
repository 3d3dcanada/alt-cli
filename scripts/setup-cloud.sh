#!/usr/bin/env bash
set -euo pipefail

# Run from any directory. Installs only in /workspace; no credentials required.
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
export CARGO_HOME=/workspace/.cargo
export RUSTUP_HOME=/workspace/.rustup
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export PATH="/workspace/.cargo/bin:/workspace/.alt-tools:$PATH"
export PYTHONPATH="/workspace/.alt-tools/python${PYTHONPATH:+:$PYTHONPATH}"
mkdir -p /workspace/.alt-tools "$CARGO_HOME" "$RUSTUP_HOME"

if [[ ! -x "$CARGO_HOME/bin/rustup" ]]; then
  python3 - <<'PY'
import hashlib
from pathlib import Path
import urllib.request
url = 'https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init'
data = urllib.request.urlopen(url).read()
expected = urllib.request.urlopen(url + '.sha256').read().decode().split()[0]
assert hashlib.sha256(data).hexdigest() == expected, 'rustup checksum mismatch'
path = Path('/workspace/.alt-tools/rustup-init')
path.write_bytes(data)
path.chmod(0o755)
PY
  /workspace/.alt-tools/rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.99.0 --component rustfmt,clippy
fi
rustup toolchain install 1.99.0 --profile minimal --component rustfmt,clippy

python3 - <<'PY'
import hashlib
import io
from pathlib import Path
import tarfile
import urllib.request
binary = Path('/workspace/.alt-tools/goose')
binary_sha = '59655719cd9b098dab59b6e2c50f1e565a935992fae2e539f52018a960e0512a'
archive_sha = 'deb2191a6b75acc0a20232fc5c52655ea2f9cc8fa2f5dffc8622e8d378a915dc'
if not binary.exists() or hashlib.sha256(binary.read_bytes()).hexdigest() != binary_sha:
    url = 'https://github.com/aaif-goose/goose/releases/download/v1.53.0/goose-x86_64-unknown-linux-gnu.tar.gz'
    data = urllib.request.urlopen(url).read()
    assert hashlib.sha256(data).hexdigest() == archive_sha, 'Goose archive checksum mismatch'
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        member = next(m for m in archive if m.isfile() and m.name == './goose')
        executable = archive.extractfile(member).read()
    assert hashlib.sha256(executable).hexdigest() == binary_sha, 'Goose binary checksum mismatch'
    binary.write_bytes(executable)
    binary.chmod(0o755)
PY

cd "$repo_dir"
python3 -m pip install --target /workspace/.alt-tools/python -r requirements-dev.txt --upgrade
cargo build --locked
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
python3 scripts/smoke_tui.py
python3 scripts/smoke_task_tui.py
python3 scripts/smoke_workbench_tui.py
python3 scripts/smoke_verification_tui.py
python3 scripts/smoke_practice_tui.py
python3 scripts/smoke_harness_tui.py
python3 scripts/acceptance_projects.py
python3 scripts/acceptance_v5.py
python3 scripts/test_research_adapters.py -v
python3 scripts/test_campaign.py -v
python3 -m unittest discover -s training/tests -v
python3 scripts/smoke_skills.py
python3 scripts/smoke_analysis.py
python3 scripts/smoke_native.py
python3 scripts/smoke_acceptance_collection.py
python3 scripts/smoke_goose.py --goose /workspace/.alt-tools/goose
python3 scripts/smoke_goose.py --goose /workspace/.alt-tools/goose --provider ollama
python3 scripts/smoke_candidates.py --goose /workspace/.alt-tools/goose
python3 scripts/smoke_stream_recovery.py --engine /workspace/.alt-tools/goose --rounds 2 --output /workspace/.alt-tools/stream-recovery.json
