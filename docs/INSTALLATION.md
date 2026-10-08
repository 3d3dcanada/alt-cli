# Install Alt on Linux

The app is **Alt**; **Alt CLI** is the repository and package name. Once installed
on your shell's PATH, type `alt` and press Enter to open the full terminal
workspace. You do not need to type `alt tui`, hold the Alt key, or choose a model
before opening it. Press **Ctrl+Q** to exit.

The supported package target is Linux x86_64. Start with CPU inference or a model
server you already use; a GPU is not required to open Alt. Model weights and the
agent engine are installed separately, after you choose them in the interface.

Use [Alt v0.6.0-beta.3](https://github.com/3d3dcanada/alt-cli/releases/tag/v0.6.0-beta.3)
for the current packaged beta, including the repair/recovery PC recorder. Its tag
appears in the TUI header and `alt build-info`; ordinary `alt --version` reports
the package version.

On the final-pass source branch, the header shows the version; **F1 → About Alt**
and `alt build-info` show the complete build identity. Its latest interface and
launch changes are documented in the [UX walkthrough](UX_FINALIZATION.md).

## Build from GitHub

On Debian/Ubuntu, install the build and managed-runtime prerequisites:

```bash
sudo apt update
sudo apt install git build-essential pkg-config curl ca-certificates python3 libgomp1
```

Install Rust using the instructions at [rustup.rs](https://rustup.rs/). Open a new
terminal afterward, or load its environment with `source "$HOME/.cargo/env"`.
The repository's `rust-toolchain.toml` selects Rust 1.99.0; an older distribution
Rust package may not meet that requirement.

```bash
git clone --branch build/final-pass-2026-10-08 https://github.com/3d3dcanada/alt-cli.git
cd alt-cli
cargo build --locked --release
./target/release/alt
```

Use `CARGO_BUILD_JOBS=2 cargo build --locked --release` to reduce parallel build
memory on smaller computers. Dependencies and the pinned toolchain require Internet
access on the first build. Subsequent launches use the built executable directly.
Do not use `scripts/setup-cloud.sh` for an ordinary desktop setup; it intentionally
installs tools under `/workspace` for the prepared cloud environment.

To make `alt` available from other project folders:

```bash
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/alt "$HOME/.local/bin/alt"
export PATH="$HOME/.local/bin:$PATH"
alt
```

The PATH change applies to this terminal. Add that same `export` to your shell's
startup configuration if `~/.local/bin` is not already there. This copies the
executable only; keep the checkout for its documentation and future builds.
For Bash the startup file is usually `~/.bashrc`; for zsh it is `~/.zshrc`.
In Fish, use `set -gx PATH "$HOME/.local/bin" $PATH` and keep the setting in
`~/.config/fish/config.fish` instead.

## Download a versioned beta

Open [GitHub Releases](https://github.com/3d3dcanada/alt-cli/releases), choose the
beta you want, and download its archive, checksum and Sigstore bundle. No Rust
compiler is needed. The release notes contain exact verification commands.
Recent GitHub CLI versions support `gh attestation verify`; its signer workflow
and source tag must match this repository's `publish.yml` and the selected tag.
This verifies GitHub-signed build provenance without a shared long-lived key.

The application archive includes offline user help and the PC recorder. The complete historical research and raw evidence are preserved separately in `research-0.6.0.tar.gz`, its checksum and `research-0.6.0-index.json`; `RESEARCH.json` in the application binds the exact research archive/index hashes and source identity. Verify the research archive’s matching release attestation before extracting it alongside the application if you want the complete evidence links offline.

After verification, extract the archive and run `bash install.sh`. Its default
prefix is `~/.local`. For custom state pass `--data-dir /your/alt/state` so the
installer backs up the folder you actually use. Start with **Try a practice
project** on Home, then follow [PC testing](PC_TESTING.md).

The installer prints a launch command for your installation. If its `bin` folder
is missing from PATH, it gives the exact PATH setting to copy into your terminal.
If another command named `alt` comes first, it identifies that path and gives the
full path to this Alt executable. It never edits your shell configuration. Shell
aliases and functions can also override `alt`; use the printed full path if you
have one. With custom state, keep the printed `--data-dir` argument when launching;
an exported `ALT_DATA_DIR` setting also selects that folder. The printed argument
works in new terminals without relying on a previous environment setting.

## Download a CI package

Open [Actions → Verify Alt](https://github.com/3d3dcanada/alt-cli/actions/workflows/verify.yml),
select a successful run for the commit you want, and download the
**linux-build-and-evidence** artifact. GitHub may require sign-in. Artifacts exist
only after a successful build and are subject to GitHub's retention period.
They are development packages, not signed production releases.

Extract the artifact ZIP, then verify and open the contained package:

```bash
sha256sum -c alt-0.6.0-linux-x86_64.tar.gz.sha256
tar -xzf alt-0.6.0-linux-x86_64.tar.gz
cd alt-0.6.0-linux-x86_64
./install.sh
~/.local/bin/alt
```

The installer needs Python 3.8 or newer. Its default prefix is `~/.local`; set `ALT_PREFIX`
to choose another prefix. It verifies every packaged file, stages the complete installation, then changes one atomic pointer for both executable and documentation. It preserves the previous complete generation. The package includes build identity,
platform requirements, a software bill of materials, documentation and licenses.
SHA256 verifies consistency with the accompanying checksum; it is not a publisher
signature. Published betas additionally provide GitHub provenance attestations. The separate private-key candidate workflow uses a trusted public key.

## First launch

1. In Home, choose **Try a practice project** for a guided repair/check/undo
   example, or choose the folder you want to work on.
2. Choose **Connect a model**. For an existing server, enter its address, test the
   connection, choose a listed model and save. Ollama uses its server root;
   compatible APIs usually use a `/v1` address.
3. Alternatively, open **Models** to import a GGUF or search Hugging Face. Inspect
   the file size and license. An import references your original file; it does
   not move or copy it. Split GGUF models need their complete numbered set.
4. Install the agent engine when Home offers it. For managed GGUF inference,
   also install the CPU runtime from Models, or select your own executable in
   Settings. Downloads are pinned and checked before use.
5. Choose access in Settings and send a request. Open Task to review actual edits
   and checks. **Prepare project checks** on Home offers commands discovered
   from the project; review them before adding. Configure contracts directly in
   Task when the model cannot do it.

Guided checks require working Bubblewrap/user namespaces. On Debian/Ubuntu,
`sudo apt install bubblewrap` installs the optional isolation tool. If your host
blocks namespaces, Alt reports that condition; choose another access mode only
if that is how you intend commands to run.

## Runtime and hardware requirements

| Component | Requirement |
|---|---|
| Portable Alt frontend | Linux x86_64, glibc 2.30 or newer; tested on Debian 11 and Ubuntu 24.04 |
| A host-built Alt executable | The system ABI it was built against; it can require a newer glibc |
| Managed Goose 1.53.0 | glibc 2.28 or newer |
| Managed llama.cpp b11429 CPU build | glibc 2.34 or newer and its required system libraries, including OpenMP |
| Package installer | Bash, Python 3 and standard Linux file utilities |
| GPU inference | A separately selected runtime compatible with the actual GPU and driver |

On minimal Debian/Ubuntu, `libgomp1` provides `libgomp.so.1`. Some custom runtimes
also need `libnuma1`. A frontend running on an older distro does not mean a newer
runtime binary will load there; use a compatible custom build or an external server.

For the GTX 1070, use a Pascal `sm_61` compatible CUDA 12 build. The default runtime
is CPU only. See [Models and hardware](WORKSPACE_GUIDE.md#models-and-hardware) for
the build recipe and [Qualification](QUALIFICATION.md) for measurements. Physical
GTX 1070 performance is still unmeasured. Begin with modest context and batch sizes.

## State, updates and uninstalling

State lives in `$XDG_DATA_HOME/alt-cli` or `~/.local/share/alt-cli`. Override it:

```bash
alt --data-dir /path/to/my-alt-state
```

A project folder and an Alt state folder are different. State holds configuration,
conversation/check evidence, journals and model-library records. Core backups preserve configuration, databases, checkpoints, execution logs/recovery receipts, inference cost/integrity receipts and archive indexes. Large raw inference requests/responses and inference archives are separate exports; core backups explicitly list these exclusions. They do not include all project files, model weights or arbitrary command effects.

Before upgrading, stop active work and make a backup outside the state directory:

```bash
alt state backup "$HOME/alt-state-backup.tar.gz"
```

Use a new filename if that backup already exists. For a source installation:

```bash
cd /path/to/alt-cli
git status
git pull --ff-only origin main
cargo build --locked --release
install -m 755 target/release/alt "$HOME/.local/bin/alt"
```

Preserve local edits before pulling. For a package installation, run the new
package's installer. It uses the verified new executable's schema-independent
backup command before activation, so an older backup-size limit does not prevent
upgrading. The resulting backup retains the old database schemas. A failed backup
or staging operation leaves the active generation intact.

Installations are stored under `~/.local/share/alt/installations`; stable executable
and documentation links both resolve through `~/.local/share/alt/current`. A legacy
installation is preserved as a complete prior generation before conversion. The
installer keeps a recovery receipt at `~/.local/share/alt/installation.json`.
After any interruption, inspect the actual pointer and verify its files:

```bash
bash install.sh --status
```

Repeating the installer resumes a prepared installation safely. `bash install.sh
--rollback` restores the prior **executable and documentation** generation; the
underlying state folder is kept intact. **Rollback does not downgrade state.**
Restore the matching pre-upgrade backup into a new directory and retain the newer
state for recovery:

```bash
alt state restore "$HOME/alt-state-backup.tar.gz" /path/to/new-restored-state
alt --data-dir /path/to/new-restored-state
```

Backups briefly acquire an exclusive generation barrier while freezing coordinated
configuration/database/recovery writes, then release it before compression. If
checks or edits remain active, the operation returns a clear retry message instead
of publishing inconsistent state. Stop work and retry; never delete journals to
unblock a backup. The manifest remains compatible with format-1 readers and adds
explicit generation/exclusion metadata. Older tools may retain their own restore
size limits; the new executable can restore a retained old-schema backup into a
new folder before launching the older executable there.

To archive growing inference payloads without losing receipts, use **Storage and
recovery → Clean old reports and finished jobs**. The preview identifies inactive
connections and their reclaimable bytes. Applying retention exports each eligible
connection to a checksummed archive before removing its bulky live payloads;
active connections and all cost/integrity receipts stay intact. Keep those archives
with your backups, or move verified archives to another storage location. Archives
are not automatically deleted. Legacy connections without activity leases are
protected until explicitly exported after old Alt processes have stopped.

To uninstall a generation-based package, remove its `bin/alt`, `bin/alt.previous` and `share/doc/alt` links and the `share/alt/installations`, `share/alt/current` and `share/alt/previous` entries under your prefix. Retain `share/alt/backups` and the recovery receipt until you no longer need recovery. For an older direct installation, remove the executable and its documentation directory. Keep the state directory
and original model files unless you also intend to remove that data. Alt does not
remove separately installed Ollama, Goose, llama.cpp or system packages.

See [Troubleshooting](TROUBLESHOOTING.md) if building, connecting or loading fails.
