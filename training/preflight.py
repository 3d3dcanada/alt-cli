#!/usr/bin/env python3
"""Record local training capability without installing packages/loading weights."""
import argparse
import importlib.metadata
import importlib.util
import json
import os
import platform
import shutil
from datetime import datetime, timezone
from pathlib import Path


def read_limit(path):
    try:
        return Path(path).read_text().strip()
    except OSError:
        return None


def capability():
    libraries = {}
    for name in ("torch", "transformers", "peft", "accelerate", "bitsandbytes", "safetensors"):
        try:
            libraries[name] = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            libraries[name] = None
    quota = read_limit("/sys/fs/cgroup/cpu.max")
    cpu_quota = None
    if quota and quota.split()[0] != "max":
        total, period = map(int, quota.split())
        cpu_quota = total / period
    available = None
    memory_info = read_limit("/proc/meminfo") or ""
    for line in memory_info.splitlines():
        if line.startswith("MemAvailable:"):
            available = int(line.split()[1]) * 1024
    report = {
        "schema_version": 1,
        "recorded_at_utc": datetime.now(timezone.utc).isoformat(),
        "platform": platform.system() + " " + platform.machine(),
        "python": platform.python_version(),
        "cpu_affinity_count": len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count(),
        "cpu_quota_cores": cpu_quota,
        "cgroup_memory_limit_bytes": read_limit("/sys/fs/cgroup/memory.max"),
        "cgroup_memory_current_bytes": read_limit("/sys/fs/cgroup/memory.current"),
        "host_available_memory_bytes": available,
        "nvidia_devices": sorted(str(p) for p in Path("/dev").glob("nvidia[0-9]*")),
        "dri_devices": sorted(str(p) for p in Path("/dev/dri").glob("*")),
        "kfd_present": Path("/dev/kfd").exists(),
        "nvidia_smi_present": shutil.which("nvidia-smi") is not None,
        "rocm_tool_present": shutil.which("rocminfo") is not None,
        "libraries": libraries,
        "training_run_performed": False,
        "note": "Filesystem/library probe only. A present device does not establish CUDA usability, free VRAM or training fit. No weights loaded.",
    }
    if libraries["torch"] and importlib.util.find_spec("torch"):
        try:
            import torch
            report["torch_cuda_available"] = torch.cuda.is_available()
            if torch.cuda.is_available():
                report["cuda_devices"] = [
                    {"name": torch.cuda.get_device_name(i), "capability": list(torch.cuda.get_device_capability(i)), "total_vram_bytes": torch.cuda.get_device_properties(i).total_memory}
                    for i in range(torch.cuda.device_count())
                ]
        except (ImportError, RuntimeError, OSError) as error:
            report["torch_probe_error"] = type(error).__name__
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = json.dumps(capability(), indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("x", encoding="utf-8") as stream:
            stream.write(result)
    print(result, end="")


if __name__ == "__main__":
    main()
