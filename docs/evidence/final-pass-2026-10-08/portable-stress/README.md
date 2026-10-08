# Portable stress checks — 2026-10-08

Both final checks passed against the unchanged portable binary SHA-256
`54b0cf02702df83afa96c3c31b5905e2c5311d787b9400f7ac9ac745e0486e51`.
Commands, script digests and exit statuses are in `validation-receipt.json`.

| Check | Result | Observations |
| --- | --- | --- |
| PTY lifecycle soak | 30/30 passed | Start, resize, input, receipt, stop, restart and cleanup; zero remaining owned processes after each round |
| Keyboard pressure journey | 30/30 passed | Ten journeys each at 120×40, 80×24 and 60×18; project-lock contention, Unicode draft, cancel, file navigation and terminal restoration |

Soak observed a maximum 14 file descriptors and 9,928,704 RSS bytes per sampled
owned process. Final retained fixture state was 48,280 bytes. These are samples,
not memory peaks. The harness uses a Linux child subreaper to emulate a proper
init for its own test descendants.

Pressure input-to-visible latency was median 38.4 ms, p95 48.7 ms and maximum
51.8 ms. Cancel-to-visible latency was median 59.7 ms, p95 65.5 ms and maximum
74.6 ms. These observations include rendering and Python screen inspection under
concurrent cloud activity; they are not GTX 1070 measurements or human usability
results. Neither check launched a model.

The initial pressure attempt is retained as `pressure-navigation-failed.*`:
nine completed journeys failed the Files assertion before the harness was
interrupted (exit 130). Its obsolete Enter keystroke tried to dismiss a cancellation
status message, but Enter correctly submitted the draft and opened model setup.
The script now verifies cancellation and draft preservation without that Enter,
then still requires file navigation, preserved draft on return, successful exit
and restored terminal settings. No application code or portable binary changed
between the failed attempt and passing rerun.
