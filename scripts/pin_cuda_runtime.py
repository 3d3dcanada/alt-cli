#!/usr/bin/env python3
"""A08 (owner PC): package a verified CUDA 12 llama-server build as a pinned Alt runtime.

Usage: pin_cuda_runtime.py BUILD_BIN_DIR OUTPUT_DIR [--name llama-cuda12-<commit>] [--bundle-cuda-libs]

BUILD_BIN_DIR is the directory holding llama-server, e.g. ~/llama-cuda12/build/bin from
scripts/build-pascal-runtime.sh. The archive is deterministic (sorted, mtime 0, uid 0).
It prints the sha256/size and the exact `pub const CUDA12` value for src/runtime.rs.
Upload the archive as a release asset of alt-cli, then paste the pin and open a PR.

Licensing (recheck before distributing, as AGENTS.md requires): llama.cpp is MIT. Shared
CUDA libraries (libcudart, libcublas, libcublasLt) are NVIDIA-redistributable only under
the CUDA EULA; --bundle-cuda-libs copies them and records that in NOTICE-CUDA.txt.
Without it, users need a CUDA 12 runtime installed system-wide.
"""
import argparse, gzip, hashlib, io, os, re, shutil, subprocess, sys, tarfile, tempfile
from pathlib import Path

CUDA_LIBS = re.compile(r"lib(cudart|cublas|cublasLt)\.so\.12")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("build", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--name")
    parser.add_argument("--bundle-cuda-libs", action="store_true")
    parser.add_argument("--url-base", default="https://github.com/3d3dcanada/alt-cli/releases/download/RUNTIME_TAG")
    args = parser.parse_args()
    server = args.build / "llama-server"
    if not server.is_file() or not os.access(server, os.X_OK):
        sys.exit(f"{server} is not an executable llama-server")
    devices = subprocess.run([str(server), "--list-devices"], capture_output=True, text=True, timeout=30)
    if "CUDA" not in devices.stdout + devices.stderr:
        print("warning: --list-devices reported no CUDA device on this machine:\n" + devices.stdout + devices.stderr, file=sys.stderr)
    name = args.name or "llama-cuda12-" + hashlib.sha256(server.read_bytes()).hexdigest()[:12]
    files = sorted(p for p in args.build.iterdir() if p.is_file() and (p.name == "llama-server" or ".so" in p.name))
    ldd = subprocess.run(["ldd", str(server)], capture_output=True, text=True).stdout
    cuda = [Path(m.split("=>")[1].split()[0]) for m in ldd.splitlines() if "=>" in m and CUDA_LIBS.search(m) and m.split("=>")[1].strip().startswith("/")]
    notice = None
    if cuda and args.bundle_cuda_libs:
        files += sorted(set(p.resolve() for p in cuda))
        notice = ("CUDA runtime libraries in this archive are redistributed under the NVIDIA CUDA Toolkit EULA "
                  "(Attachment A, redistributable software): " + ", ".join(p.name for p in cuda) + "\n")
    elif cuda:
        print("note: llama-server links " + ", ".join(p.name for p in cuda) + " dynamically; users need a CUDA 12 runtime, "
              "or rerun with --bundle-cuda-libs after checking the EULA.", file=sys.stderr)
    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output / f"{name}.tar.gz"
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.GNU_FORMAT) as tar:
        entries = [(f.name if f.parent == args.build else Path(f).name, f.read_bytes(), 0o755 if f.name == "llama-server" else 0o644) for f in files]
        entries.append(("NOTICE.txt", b"llama.cpp (MIT): https://github.com/ggml-org/llama.cpp\nBuilt with CUDA 12 for compute capability 6.1+.\n", 0o644))
        if notice:
            entries.append(("NOTICE-CUDA.txt", notice.encode(), 0o644))
        for member, data, mode in sorted(entries):
            info = tarfile.TarInfo(f"{name}/{member}")
            info.size, info.mode, info.mtime, info.uid, info.gid, info.uname, info.gname = len(data), mode, 0, 0, 0, "", ""
            tar.addfile(info, io.BytesIO(data))
    with open(archive, "wb") as handle, gzip.GzipFile(fileobj=handle, mode="wb", mtime=0, filename="") as gz:
        gz.write(raw.getvalue())
    data = archive.read_bytes()
    sha = hashlib.sha256(data).hexdigest()
    print(f"archive: {archive}\nsha256:  {sha}\nsize:    {len(data)}\n")
    print("Paste into src/runtime.rs (replace the `None`):\n")
    print(f'pub const CUDA12: Option<Pin> = Some(Pin {{\n    label: "llama.cpp CUDA 12 ({name})",\n    url: "{args.url_base}/{archive.name}",\n    sha256: "{sha}",\n    size: {len(data)},\n    directory: "{name}",\n}});')


if __name__ == "__main__":
    main()
