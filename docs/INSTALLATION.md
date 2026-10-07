# Install Alt on Linux

The supported package target is Linux x86_64. Start with CPU inference or a model
server you already use; a GPU is not required to open Alt. Model weights and the
agent engine are installed separately, after you choose them in the interface.

Use [Alt v0.6.0-beta.2](https://github.com/3d3dcanada/alt-cli/releases/tag/v0.6.0-beta.2)
for the current packaged beta, including the repair/recovery PC recorder. Its tag
appears in the TUI header and `alt build-info`; ordinary `alt --version` reports
the package version.

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
git clone https://github.com/3d3dcanada/alt-cli.git
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

## Download a versioned beta

Open [GitHub Releases](https://github.com/3d3dcanada/alt-cli/releases), choose the
beta you want, and download its archive, checksum and Sigstore bundle. No Rust
compiler is needed. The release notes contain exact verification commands.
Recent GitHub CLI versions support `gh attestation verify`; its signer workflow
and source tag must match this repository's `publish.yml` and the selected tag.
This verifies GitHub-signed build provenance without a shared long-lived key.

After verification, extract the archive and run `bash install.sh`. Its default
prefix is `~/.local`. For custom state pass `--data-dir /your/alt/state` so the
installer backs up the folder you actually use. Start with **Try a practice
project** on Home, then follow [PC testing](PC_TESTING.md).

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

The installer needs Python 3. Its default prefix is `~/.local`; set `ALT_PREFIX`
to choose another prefix. It verifies every packaged file and keeps a previous
executable when replacing a different version. The package includes build identity,
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
conversation/check evidence, journals and model-library records. Backups cover Alt
state, not all project files, model weights or arbitrary command effects.

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
package's installer. It saves a pre-upgrade state backup before changing an existing version and prints its location; a failed backup aborts the update. Its `./install.sh --rollback` restores the retained previous
executable. **Executable rollback does not downgrade state.** Retain newer state
and restore the backup that matches the older executable into a new directory:

```bash
alt state restore "$HOME/alt-state-backup.tar.gz" /path/to/new-restored-state
alt --data-dir /path/to/new-restored-state
```

To uninstall, remove the executable you installed and, for a package installation,
its `share/doc/alt` directory under your chosen prefix. Keep the state directory
and original model files unless you also intend to remove that data. Alt does not
remove separately installed Ollama, Goose, llama.cpp or system packages.

See [Troubleshooting](TROUBLESHOOTING.md) if building, connecting or loading fails.
