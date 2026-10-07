# Published beta verification

[Alt v0.6.0-beta.1](https://github.com/3d3dcanada/alt-cli/releases/tag/v0.6.0-beta.1)
was published on 2026-10-07 from clean commit
`7ea6533d2acafe947d6ffdcc7e68809d94305005`.

- Public archive SHA-256: `f61dc2b54059b6b1c185540f24ca5480804c7d3253c5eb600510026eec7f1b90`.
- Installed executable SHA-256: `7017be5b3e526d53dee7b08dca16aa109d4c9244d8e9d36ecba52bdad678f4b7`.
- [Release workflow](workflow-37564733425.json), [main CI](workflow-37564712971.json)
  and [tag CI](workflow-37564733422.json) all completed successfully.
- All seven [exact-package gates](release-gate.json) passed, including previous
  version recovery and real installed-terminal checks on Debian 11/Ubuntu 24.04.
- All six downloaded assets matched their GitHub API digests; archive checksum,
  GitHub workflow identity, tag and exact source commit were independently verified.
- A fresh temporary-prefix install from that downloaded archive passed and reported
  the expected clean build. See [verification.json](verification.json) and
  [consumer-install.log](consumer-install.log).

The release attached its own [attestation verification](attestation-verification.json)
and [provenance bundle](alt-0.6.0-linux-x86_64.tar.gz.sigstore.jsonl).
[Consumer verification](consumer-attestation-verification.json) is a separate
cloud-side check, additionally enforcing the exact source commit.

## Cloud trust-data transport

The default Sigstore CDN and its documented GitHub Pages mirror returned HTTP 403
from this cloud. The initial errors are retained. GitHub's publication-side
verification had already passed through its normal trust-data path.

For independent cloud verification, the official `sigstore/root-signing`
repository was cloned at `5b812320c5ba729d33860e8afc8d19e18db02318`.
Its unchanged signed metadata and targets were served only on loopback to the
GitHub CLI TUF client. The bootstrap root was byte-for-byte present in the
checksum-verified official GitHub CLI 2.102.0 binary; its source is the CLI's pinned
`sigstore-go` 1.3.0 dependency. TUF checked the root transition, metadata signatures,
expiry and target integrity. The resulting trusted root then verified the artifact
bundle, repository, signer workflow, tag and source commit. No signature, expiry or
TLS verification was disabled. [trust-bootstrap.json](trust-bootstrap.json) records
the identities; [signed-tuf-metadata/](signed-tuf-metadata/) retains the relevant
signed bytes. This transport workaround was specific to the cloud network.

The two Python files record the exact local procedures and workspace paths.
They are evidence scripts, not an alternative installer. No new model inference
or physical hardware measurement occurred during this consumer check. These
post-publication receipts are a documentation-only follow-up; the tagged archive
necessarily predates its own download verification and remains unchanged.
