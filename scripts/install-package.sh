#!/usr/bin/env bash
set -euo pipefail
package_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
prefix="${ALT_PREFIX:-${HOME}/.local}"
command -v python3 >/dev/null || { echo 'Alt installation needs Python 3 to verify and safely replace files. Install your operating system python3 package, then run this installer again.' >&2; exit 1; }
if [[ "${1:-}" == --rollback ]]; then
  test -x "$prefix/bin/alt.previous" || { echo 'No previous executable was saved.' >&2; exit 1; }
  "$prefix/bin/alt.previous" --version
  python3 - "$prefix/bin/alt.previous" "$prefix/bin/alt" <<'PY'
import os,shutil,sys,tempfile
from pathlib import Path
source,target=map(Path,sys.argv[1:])
fd,path=tempfile.mkstemp(prefix='.alt-rollback-',dir=target.parent)
try:
 with os.fdopen(fd,'wb') as f:
  f.write(source.read_bytes());f.flush();os.fsync(f.fileno())
 os.chmod(path,0o755);os.replace(path,target)
 fd=os.open(target.parent,os.O_RDONLY)
 try:os.fsync(fd)
 finally:os.close(fd)
finally:
 if os.path.exists(path):os.unlink(path)
PY
  echo 'Previous executable restored. For a state schema downgrade, restore its pre-upgrade backup into a new folder and launch Alt with --data-dir pointing there; keep the newer state for recovery.'
  exit 0
fi
if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
  echo 'This package requires Linux x86_64.' >&2; exit 1
fi
if [[ "${1:-}" == --verify-key ]]; then
  test -n "${2:-}" || { echo 'Pass the trusted public-key file.' >&2; exit 1; }
  openssl dgst -sha256 -verify "$2" -signature "$package_dir/CONTENTS.json.sig" "$package_dir/CONTENTS.json"
fi
python3 - "$package_dir" <<'PY'
import hashlib,json,sys
from pathlib import Path
p=Path(sys.argv[1]);manifest=json.loads((p/'PLATFORM.json').read_text())
assert hashlib.sha256((p/'alt').read_bytes()).hexdigest()==manifest['sha256'],'Binary checksum failed'
contents=json.loads((p/'CONTENTS.json').read_text())
required={'alt','install.sh','README.md','LICENSE','THIRD_PARTY.md','DEPENDENCIES.md','PLATFORM.json','BUILD.json','SBOM.cdx.json'}
assert isinstance(contents,dict) and required<=contents.keys(),'Incomplete package manifest'
permitted=set(contents)|{'CONTENTS.json','CONTENTS.json.sig','PLATFORM.json.sig'}
for target in p.rglob('*'):
 assert not target.is_symlink(),'Linked package entry'
 assert target.is_dir() or (target.is_file() and str(target.relative_to(p)) in permitted),'Unexpected package entry'
for name,digest in contents.items():
 rel=Path(name);assert not rel.is_absolute() and '..' not in rel.parts,'Invalid content path'
 target=p/rel;assert target.is_file() and not target.is_symlink(),'Missing or linked package file'
 assert hashlib.sha256(target.read_bytes()).hexdigest()==digest,f'Package integrity failed: {name}'
PY
"$package_dir/alt" --version || { echo 'Incompatible binary; see PLATFORM.json.' >&2; exit 1; }
install -d "$prefix/bin" "$prefix/share/doc/alt"
python3 - "$package_dir/alt" "$prefix/bin/alt" <<'PY'
import os,sys,tempfile
from pathlib import Path
source,target=map(Path,sys.argv[1:])
def replace(path,data):
 fd,name=tempfile.mkstemp(prefix='.alt-install-',dir=path.parent)
 try:
  with os.fdopen(fd,'wb') as f:f.write(data);f.flush();os.fsync(f.fileno())
  os.chmod(name,0o755);os.replace(name,path)
 finally:
  if os.path.exists(name):os.unlink(name)
data=source.read_bytes()
if target.exists() and target.read_bytes()!=data:replace(target.with_name('alt.previous'),target.read_bytes())
replace(target,data)
fd=os.open(target.parent,os.O_RDONLY)
try:os.fsync(fd)
finally:os.close(fd)
PY
for document in README.md LICENSE THIRD_PARTY.md DEPENDENCIES.md PLATFORM.json BUILD.json SBOM.cdx.json CONTENTS.json; do
  install -m 644 "$package_dir/$document" "$prefix/share/doc/alt/$document"
done
cp -R "$package_dir/licenses" "$package_dir/docs" "$package_dir/prompts" "$prefix/share/doc/alt/"
printf 'Installed Alt. Start it with:\n  %s/bin/alt\n' "$prefix"
printf 'The previous executable, if any, is retained as alt.previous.\n'
