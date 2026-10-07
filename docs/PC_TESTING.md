# Test Alt on your computer

These checks supply the physical measurements unavailable in the cloud. Keep
the exact model you choose; Alt does not need to switch providers to run them.

## Install and check the application

Download the Linux x86_64 package from the project's GitHub Releases page,
verify it as described in [Installation](INSTALLATION.md), extract it, and run
`bash install.sh`. No Rust compiler is needed for a packaged release. Python 3
is required for the installer, practice checks and the test recorder.

From the extracted package, run:

```bash
python3 test-my-pc.py --alt "$HOME/.local/bin/alt"
```

This records the exact build, operating system and hardware without inference.
The new `alt-pc-results-*` folder contains a summary and raw results. An error
stays in the report; the recorder does not change settings to make a check pass.

## Complete the practice task

1. Start `alt`, choose **Try a practice project** on Home, and run its check.
   Expect three failed cases out of four on the original example.
2. Connect your chosen uncensored/abliterated model. Keep the practice folder
   selected. The example request is in the message box immediately after creation;
   its README also contains the goal if you return after restarting. An existing
   unsent message is preserved instead of being replaced by the practice request.
3. Ask the model to repair the example. Inspect the edit and actual check result.
   A model saying it succeeded is not a passing behavioral check.
4. Quit, restart and return to Task. Confirm the edit and check are retained.
5. Use **Undo task**, then rerun the check. The original failure should return.
6. Repeat at your normal terminal size and at 80×24. Record confusing labels,
   invisible controls, delayed input or steps that need outside help.

Manual editing is also supported from Project files. This distinguishes problems
with the application from a selected model's ability to implement the repair.
The example uses Python's standard library and requires no package downloads.
Full access uses host permissions; Guided checks need working Bubblewrap.

## Measure the selected model

Import/select an uncensored model or configure your existing server in Alt first.
For a GTX 1070, select a runtime built with Pascal-compatible CUDA; the managed
default is CPU inference. Do not infer GPU use from the presence of an NVIDIA card.

```bash
python3 test-my-pc.py --alt "$HOME/.local/bin/alt" --live --contexts 2048,4096,8192
```

Use `--data-dir /your/alt/state` or `--profile NAME` if you use a custom setup.
The live run measures load/generation and cancellation/restart where Alt owns the
runtime, then probes native tool use. External runtime lifecycle is explicitly
unmeasured. A failed connection or inadequate memory remains a failed attempt.
These commands can take several minutes per context on an older CPU.

Afterward, try a real task in a copy of one of your projects. Choose **Prepare
project checks** on Home, select the actual test command, and configure its
purpose/evidence in Task. Record the model filename and quantization, context,
GPU layers, time to first response, whether the requested change works, and
whether cancellation releases resources. Test 16K context only after smaller
windows fit comfortably; retrieval does not enlarge the model's native window.

## Update and recover

Stop active work before an update. The package installer preserves the previous
executable and backs up the selected state folder before changing versions.
For custom state use `bash install.sh --data-dir /your/alt/state`.

If you roll back with `bash install.sh --rollback`, keep the newer state and
restore the matching pre-upgrade backup into a new folder. Launch with
`alt --data-dir /restored/folder`. The installer prints the backup location.
An executable rollback does not downgrade a database schema.

Review reports before sharing them: model endpoints, machine details and local
paths may be present. Nothing is uploaded automatically. Include the exact
release version and observed steps with a GitHub compatibility report.
