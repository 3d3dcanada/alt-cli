#!/usr/bin/env python3
"""Opt-in real-model evaluation. Requires a deliberately selected uncensored GGUF.

Uses Alt's managed CPU runtime and real Goose. Tools are explicitly authorized
inside a newly created disposable project. No model is silently downloaded.
Artifacts and observed failures are kept in --output for review.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--model", type=Path, required=True)
parser.add_argument("--sha256", required=True)
parser.add_argument("--uncensored", action="store_true", required=True,
                    help="Confirm this is the uncensored/abliterated checkpoint chosen for evaluation")
parser.add_argument("--engine", type=Path, required=True)
parser.add_argument("--runtime", type=Path, required=True)
parser.add_argument("--binary", type=Path, default=Path(__file__).resolve().parents[1] / "target/release/alt")
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
root = args.output.resolve()
root.mkdir(parents=True, exist_ok=False)
project = root / "project"
project.mkdir()
state = root / "state"
state.mkdir()
binary = args.binary.resolve()
base = [str(binary), "--data-dir", str(state), "--engine", str(args.engine.resolve())]
# JSON strings are valid TOML basic strings for these ordinary absolute paths.
(state / "preferences.toml").write_text(
    f"project = {json.dumps(str(project))}\n"
    f"runtime_path = {json.dumps(str(args.runtime.resolve()))}\n"
    "context_tokens = 8192\nmax_turns = 12\n")
original = """def total(items):
    return sum(items) + 1

if __name__ == '__main__':
    assert total([2, 3, 5]) == 10
    assert total([]) == 0
    print('ALT_MANAGED_CHECK_OK')
"""
(project / "calculator.py").write_text(original)
before = subprocess.run(["python3", "calculator.py"], cwd=project, capture_output=True, text=True)
assert before.returncode != 0
(root / "before.txt").write_text(before.stderr)
imported = subprocess.run(base + ["models", "import", str(args.model.resolve()), "--uncensored", "--use"],
                          cwd=project, capture_output=True, text=True, timeout=240)
(root / "import.log").write_text(imported.stderr)
assert imported.returncode == 0, imported.stderr
artifact = json.loads(imported.stdout)
assert artifact["sha256"] == args.sha256, "Wrong evaluation checkpoint"
(root / "artifact.json").write_text(json.dumps(artifact, indent=2) + "\n")

def descendants(pid):
    # Some cloud kernels omit /proc/PID/task/TID/children. Build the tree from
    # PPid in status, which also handles children spawned by worker threads.
    parents = {}
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit(): continue
        try:
            lines = (entry / 'status').read_text().splitlines()
            parent = next(int(line.split()[1]) for line in lines if line.startswith('PPid:'))
            parents[int(entry.name)] = parent
        except (OSError, StopIteration):
            continue
    found = set()
    frontier = {pid}
    while frontier:
        children = {child for child, parent in parents.items() if parent in frontier} - found
        found.update(children)
        frontier = children
    return found

def turn(name, prompt, resume=None):
    command = base + ["run", prompt, "--allow-tools", "--json", "--timeout", "600"]
    if resume:
        command += ["--resume", resume]
    seen = set()
    peak_rss = 0
    started = time.monotonic()
    with (root / f"{name}.jsonl").open("w") as out, (root / f"{name}.stderr").open("w") as err:
        process = subprocess.Popen(command, cwd=project, stdout=out, stderr=err)
        try:
            while process.poll() is None:
                children = descendants(process.pid)
                seen.update(children)
                rss = 0
                for pid in children | {process.pid}:
                    try:
                        status = Path(f"/proc/{pid}/status").read_text()
                        rss += next(int(line.split()[1]) * 1024 for line in status.splitlines() if line.startswith("VmRSS:"))
                    except (OSError, StopIteration):
                        pass
                peak_rss = max(peak_rss, rss)
                if time.monotonic() - started > 780:
                    raise TimeoutError("Managed evaluation exceeded 13 minutes")
                time.sleep(0.2)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)
    assert process.returncode == 0, (root / f"{name}.stderr").read_text()
    assert len(seen) >= 2, "Process instrumentation missed the engine/runtime"
    events = [json.loads(line) for line in (root / f"{name}.jsonl").read_text().splitlines()]
    assert events[-1]["type"] == "turn_end"
    calls = [e["data"]["update"] for e in events if e["type"] == "update"
             and e["data"]["update"].get("sessionUpdate") == "tool_call"]
    results = [e["data"]["update"].get("rawOutput", {}) for e in events if e["type"] == "update"]
    assert any(r.get("exit_code") == 0 and "ALT_MANAGED_CHECK_OK" in r.get("stdout", "") for r in results), "No successful tool check observed"
    # Normal completion must not leave Alt's local server or agent behind.
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline and any(Path(f"/proc/{pid}").exists() for pid in seen):
        time.sleep(0.1)
    assert not any(Path(f"/proc/{pid}").exists() for pid in seen), "An owned child process survived shutdown"
    return events[0]["session"]["id"], {
        "elapsed_seconds": round(time.monotonic() - started, 2),
        "sampled_process_tree_peak_rss_bytes": peak_rss,
        "tool_calls": [{"title": c.get("title"), "input": c.get("rawInput")} for c in calls],
        "owned_processes_stopped": True,
        "observed_child_process_count": len(seen),
    }

session, repair = turn("repair", "Read calculator.py. Run it to observe the assertion failure. Fix the total function without changing or removing the assertions or success message. Run python3 calculator.py again and report its actual output.")
changed = (project / "calculator.py").read_text()
assert changed != original
for line in original.splitlines():
    if "assert " in line or "print(" in line:
        assert line in changed, "The model changed the independent check"
after = subprocess.run(["python3", "calculator.py"], cwd=project, capture_output=True, text=True)
assert after.returncode == 0 and "ALT_MANAGED_CHECK_OK" in after.stdout
(root / "after.txt").write_text(after.stdout)
resumed, recovery = turn("resume", "Run python3 calculator.py again in the same project and report its exact output. Do not change any files.", session)
assert resumed == session
assert (project / "calculator.py").read_text() == changed
report = {"checkpoint": artifact["name"], "sha256": artifact["sha256"],
          "uncensored_status": "publisher claim, explicitly selected for evaluation",
          "context_tokens": 8192, "mode": "managed llama.cpp CPU", "session": session,
          "repair": repair, "resume": recovery, "independent_check": "passed",
          "gtx_1070_tested": False}
(root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
