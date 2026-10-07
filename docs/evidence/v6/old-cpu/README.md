# Emulated older CPU probes

These are instruction-compatibility probes, not physical hardware qualification.
The tested Alt binary SHA-256 is
`e8611c9b3e75dd145578c6fb62ca5463b1e411f6ba5aa09606e978fe2ab83d5c`,
with compiled source fingerprint
`471c9abccc5a84766fe8467905fd44aec7153367954c78965be5d2fcd57259d9`.
Later practice-dialog wording is not part of this binary. Goose and llama.cpp
hashes and the exact selected uncensored Spark weight hash are in each report.
No model switching occurred.

[Debian / QEMU 10.0.13](debian-qemu10/report.json) passed 14 probes: emulator
version, Alt version and practice creation, Goose version, and llama.cpp version
on each of `Nehalem`, `Penryn` and `qemu64`, plus one Nehalem raw-completion
request producing eight tokens. All three profiles omit AVX. The prompt's literal
requested answer is **not** a success criterion; generated token count checks
that model execution occurred. Raw text, timings and runtime logs are retained.
This is not a coding score, a chat-template test, an engine/tool-use test under
emulation or a physical throughput result. All packages/runtimes were mounted
read-only; the container had no external network access.

The first [Ubuntu / QEMU 8.2 attempt](ubuntu-qemu8/report.json) failed when
starting the Rust executables. A trace ended in the emulator's internal SIGSEGV
while opening `/proc/self/maps`. Independent `cat /proc/self/maps` controls
reproduced the emulator fault both with `Nehalem` and `max`; native `cat` passed.
Those controls were terminated after their recorded timeout. The runtime's
Nehalem generation probe nevertheless completed. Failed evidence is retained;
it is not counted as an application compatibility pass. Retrying with the newer
emulator resolved the startup failures without changing Alt, Goose or the model.

Each folder contains the exact probe script, test-image Dockerfile and local
image ID. Base image digests are pinned in the Dockerfiles; installed package
versions include the emulator version in the report. The emulator image and its
binaries are test tools and are not distributed with Alt. The frontend's OS ABI
and managed-runtime requirements remain those in the installation guide.
