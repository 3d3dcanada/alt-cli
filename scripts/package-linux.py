#!/usr/bin/env python3
"""Create a local, unpublished Linux package with dependency license notices."""
import argparse
import gzip
import os
import re
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
from build_provenance import verify, sbom

repo = Path(__file__).resolve().parents[1]
assert platform.system() == "Linux" and platform.machine() == "x86_64"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--binary", type=Path)
args = parser.parse_args()
if args.binary is None:
    subprocess.run(["cargo", "build", "--locked", "--release"], cwd=repo, check=True)
binary = (args.binary or repo / "target/release/alt").resolve()
versions = re.findall(r"GLIBC_([0-9.]+)", subprocess.check_output(["readelf", "-V", str(binary)], text=True))
minimum = max(set(versions), key=lambda s: tuple(map(int, s.split('.'))))
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", "x86_64-unknown-linux-gnu"], cwd=repo))
active = {node["id"] for node in metadata["resolve"]["nodes"]}
version = tomllib.loads((repo / "Cargo.toml").read_text())["package"]["version"]
assert subprocess.check_output([str(binary), "--version"], text=True).strip() == "alt " + version, "Binary version does not match source manifest"
build = verify(repo,binary)
name = f"alt-{version}-linux-x86_64"
dist = repo / "dist"
dist.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix="alt-package-") as temp:
    package = Path(temp) / name
    package.mkdir()
    shutil.copy2(binary, package / "alt")
    (package / "PLATFORM.json").write_text(json.dumps({"os":"linux","arch":"x86_64","frontend_glibc_minimum":minimum,"managed_engine_glibc_minimum":"2.28","managed_runtime_glibc_minimum":"2.34","cpu":"x86-64 baseline; runtime chooses a supported CPU variant","sha256":hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2)+"\n")
    (package/'BUILD.json').write_text(json.dumps(build,sort_keys=True,indent=2)+'\n')
    (package/'SBOM.cdx.json').write_text(json.dumps(sbom(repo,metadata,build,hashlib.sha256(binary.read_bytes()).hexdigest()),sort_keys=True,indent=2)+'\n')
    if os.environ.get('ALT_SIGNING_KEY'):
        subprocess.run(['openssl','dgst','-sha256','-sign',os.environ['ALT_SIGNING_KEY'],'-out',str(package/'PLATFORM.json.sig'),str(package/'PLATFORM.json')],check=True)
    for doc in ["README.md", "LICENSE", "THIRD_PARTY.md"]:
        shutil.copy2(repo / doc, package / doc)
    shutil.copytree(repo / "docs", package / "docs")
    shutil.copytree(repo / "prompts", package / "prompts")
    shutil.copy2(repo / "scripts/install-package.sh", package / "install.sh")
    shutil.copy2(repo / "scripts/test-my-pc.py", package / "test-my-pc.py")
    (package / "install.sh").chmod(0o755)
    licenses = package / "licenses"
    licenses.mkdir()
    manifest = ["# Dependency provenance", "", "From locked installed metadata resolved for Linux x86_64 GNU, including development dependencies.", "", "| Package | Version | Declared license | Source |", "|---|---|---|---|"]
    for item in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if item["name"] == "alt-cli" or item["id"] not in active: continue
        source = item.get("repository") or item.get("source") or "local"
        manifest.append(f"| {item['name']} | {item['version']} | {item.get('license') or 'See supplied files'} | {source} |")
        crate = Path(item["manifest_path"]).parent
        destination = licenses / f"{item['name']}-{item['version']}"
        destination.mkdir()
        candidates = [p for p in crate.iterdir() if p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "COPYRIGHT", "NOTICE"))]
        if item.get("license_file"): candidates.append(crate / item["license_file"])
        for file in candidates:
            if file.is_file(): shutil.copy2(file, destination / file.name)
            elif file.is_dir(): shutil.copytree(file, destination / file.name, dirs_exist_ok=True)
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
    rust_license = licenses / "rust-toolchain"
    rust_license.mkdir()
    for file in (sysroot / "share/doc/rust").glob("*"):
        if file.name.upper().startswith(("LICENSE", "COPYRIGHT")):
            if file.is_file(): shutil.copy2(file, rust_license / file.name)
            elif file.is_dir(): shutil.copytree(file, rust_license / file.name, dirs_exist_ok=True)
    (package / "DEPENDENCIES.md").write_text("\n".join(manifest) + "\n")
    contents={str(path.relative_to(package)):hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(package.rglob('*')) if path.is_file()}
    (package/'CONTENTS.json').write_text(json.dumps(contents,sort_keys=True,indent=2)+'\n')
    if os.environ.get('ALT_SIGNING_KEY'):
        subprocess.run(['openssl','dgst','-sha256','-sign',os.environ['ALT_SIGNING_KEY'],'-out',str(package/'CONTENTS.json.sig'),str(package/'CONTENTS.json')],check=True)
    artifact = dist / (name + ".tar.gz")
    epoch=int(os.environ.get('SOURCE_DATE_EPOCH','0'))
    def normalized(info):
        info.uid=info.gid=0;info.uname=info.gname='';info.mtime=epoch
        info.mode=0o755 if info.isdir() or info.name in [name+'/alt',name+'/install.sh'] else 0o644
        return info
    with artifact.open('wb') as raw:
        with gzip.GzipFile(filename='',mode='wb',fileobj=raw,mtime=epoch) as compressed:
            with tarfile.open(fileobj=compressed,mode='w') as archive:
                archive.add(package,arcname=name,filter=normalized)
    digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
    (dist / (name + ".tar.gz.sha256")).write_text(f"{digest}  {artifact.name}\n")
    if os.environ.get('ALT_SIGNING_KEY'):
        subprocess.run(['openssl','dgst','-sha256','-sign',os.environ['ALT_SIGNING_KEY'],'-out',str(artifact)+'.sig',str(artifact)],check=True)
    else:
        Path(str(artifact)+'.sig').unlink(missing_ok=True)
    print(artifact)
    print(digest)
