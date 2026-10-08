#!/usr/bin/env bash
set -euo pipefail
package_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
command -v python3 >/dev/null || { echo 'Alt installation needs Python 3.8 or newer.' >&2; exit 1; }
exec python3 "$package_dir/install-generation.py" --package "$package_dir" "$@"
