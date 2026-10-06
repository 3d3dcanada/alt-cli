# Third-party provenance

Alt's interface and ACP transport in this repository were written for this
project. Goose runs as a separately installed executable; no Goose source files
have been copied into the application.

| Component | Tested version | Source / license |
|---|---|---|
| Goose | 1.53.0 | [aaif-goose/goose](https://github.com/aaif-goose/goose/tree/7debb275d36559ee4e81799362e7f638275dfed3), Apache-2.0 |
| Rust | 1.99.0 | [Rust](https://www.rust-lang.org/), MIT/Apache-2.0 |
| llama.cpp CPU runtime, optional evaluation dependency | b11429, d81235049 | [llama.cpp release](https://github.com/ggml-org/llama.cpp/releases/tag/b11429), MIT |
| Rust libraries | See `Cargo.lock` | Resolved dependency versions and registry checksums are pinned in the lockfile |

The setup script pins the upstream Goose Linux x86_64 GNU artifact. Its hashes
were computed after downloading the release through verified HTTPS; these are
reproducibility pins, not an independently verified publisher signature.

- Goose archive: `deb2191a6b75acc0a20232fc5c52655ea2f9cc8fa2f5dffc8622e8d378a915dc`
- Goose executable: `59655719cd9b098dab59b6e2c50f1e565a935992fae2e539f52018a960e0512a`
- Optional llama.cpp CPU archive: `f6d25dde8f51133143d1453da4fd5f73b145127177612a283bf7995957af3392`

Before distributing a combined Alt/Goose package, include the applicable upstream
licenses/notices and dependency attributions. This repository currently builds
the Alt executable and installs Goose separately from its official release; it
does not contain a bundled redistribution. Naming Alt does not imply upstream
endorsement.

The optional local Alt package includes `DEPENDENCIES.md` generated from the
locked Linux dependency graph, plus the available crate and Rust runtime license
and notice files under `licenses/`. It contains no Goose/llama.cpp binaries or
model weights. Those are downloaded separately when the user chooses to install
them. The package is a development artifact and has not been uploaded or published.

The 0.3 tools/checkpoint layer is original Alt code using cap-std (Apache-2.0/MIT)
and similar (Apache-2.0). Exact resolved licenses are included by the packaging script.
Bubblewrap is an optional system dependency (LGPL-2.0-or-later); its binary is not
bundled in the Alt package. The isolation test used Debian's 0.8.0-2+deb12u1 package,
SHA256 `3cc9134a3286ad01a323dcd924ba123eb634cefaeec82d774257e06308aeaadb`, verified against
Debian's package index. Container images are build/test dependencies, not bundled.
