# Troubleshooting

Start with `alt --version`, `alt hardware`, and, after configuring a model,
`alt doctor`. Home and project tools remain usable when inference is disconnected.
For a report, include the operation, exact error and relevant redacted output.

| What happened | Next step |
|---|---|
| `cargo` is missing or Rust is too old | Install rustup, open a new terminal and build from the checkout. The toolchain file selects Rust 1.99.0. |
| Compilation runs out of memory | Retry with `CARGO_BUILD_JOBS=2 cargo build --locked --release`; close other heavy programs. |
| `alt` is not found after building | Run `./target/release/alt` from the checkout, or install it into a directory on PATH as described in Installation. |
| Package installer asks for Python | Install your distribution's `python3` package and rerun the installer. |
| The screen is not interactive | Open a real terminal. Use `alt plain` for simple text interaction or `alt run` for scripts. Ctrl+Q exits the TUI. |
| The state directory is unwritable | Select a writable directory with `alt --data-dir /path/to/state`. The prepared cloud uses `/workspace/.alt-data`. |
| Server has no models or cannot connect | Start the server, load the intended model and test its connection in Alt. Ollama uses the server root; compatible APIs usually need `/v1`. |
| Connection failed while composing | The draft is retained. Repair the connection, choose the intended model and send it again. |
| Agent engine is missing | Choose the Home installer or select an existing Goose executable in Settings. Engine and inference runtime are different components. |
| Managed runtime cannot load `libgomp.so.1` | Debian/Ubuntu provides it in `libgomp1`. Inspect the exact library named in the error; custom builds may also need `libnuma1`. |
| Runtime needs a newer glibc | Use a compatible runtime build or an external server. The portable frontend and downloadable runtime have different requirements. |
| GPU requested but no device is reported | Select a runtime compatible with the actual GPU/driver, or explicitly set GPU layers to 0 for CPU. Alt does not silently switch the requested configuration. |
| GTX 1070 is unsupported by a CUDA build | Pascal needs `sm_61` support; use a compatible CUDA 12 build. CUDA 13-only builds do not cover this card. |
| Available RAM is below the model size | Close other programs or explicitly choose smaller weights. Lower context/batch/cache settings help runtime overhead, but cannot make an oversized model fit. |
| Model reaches its output limit | Review its template and token use. An explicit reasoning-mode experiment can help some templates, but the measured Spark pilot did not improve correctness. |
| Model repeats calls to an unavailable tool | Inspect Settings → Tool focus. Coding needs named checks; select All/Terminal yourself if arbitrary commands are intended. |
| Guided check reports unavailable isolation | Install Bubblewrap and check whether the host allows user namespaces. Guided checks fail closed; access mode is an explicit user choice. |
| Practice check cannot start `python3` | Install your distribution’s Python 3 package, then rerun the practice check. The missing-runner error is retained as evidence. |
| No project checks are configured | Choose **Prepare project checks** on Home, inspect the suggested command and add the check you want. |
| Tests exit zero but verification fails | A Tests contract needs a fresh complete case-level report with executed passing tests. Check the selected format, report path and raw evidence. |
| Passing evidence became stale | Source, verifier, command, assertion or tracked dependencies changed. Rerun the required checks. |
| Undo refuses to overwrite a file | The file changed after the recorded edit. Review the current file/diff and preserve manual changes before deciding how to recover. |
| An operation is waiting | The project may be in use by another operation. Input remains active; Esc requests cancellation. An atomic write already committing finishes and reports its outcome. |
| An older executable rejects newer state | Keep newer state and restore the matching prior backup into a new folder. Replacing the executable does not downgrade the database. |

## Model quality versus application errors

Tool calls, successful generation and fluent summaries do not establish a correct
repair. Inspect Task's current requirements and independent check results. The
retained [evaluation report](IMPLEMENTATION.md) includes failed uncensored-model
attempts and false completion claims. Changing a tool focus or reasoning mode is
an experiment on the same model, not a guaranteed fix or automatic substitution.

## Useful details for an issue

Use the [bug-report template](https://github.com/3d3dcanada/alt-cli/issues/new/choose).
State your distribution, terminal, Alt version, CPU/RAM/GPU, connection type and
what you expected. For inference problems, include the exact model variant,
quantization, runtime version and context size. Hardware/runtime hashes and saved
qualification reports help distinguish configurations.

Review logs before sharing them: conversation exports and tool evidence can contain
project text. Never paste an API key or private project files merely to reproduce
a connection issue. The Settings diagnostics flow is available for a redacted
system report; share only the information relevant to the problem.
