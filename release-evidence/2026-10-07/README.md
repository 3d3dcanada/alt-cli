# Exact local package verification

The final local 0.6.0 archive passed all seven release-gate checks.
[release-gate.json](release-gate.json) retains every result and container identity;
[summary.json](summary.json) pins the current and actual prior archive hashes.
The checks cover installed integrity and TUI, interrupted installation, current
source verification, real prior project state, schema-3 to schema-4 migration,
rollback/re-upgrade, the official CycloneDX schema, Debian 11 and Ubuntu 24.04.

This is an unpublished, unsigned local archive. The older published beta predates
the compact repair work. Build current main to use the additions. The old and new
local harness packages both report 0.6.0; their source, binary and archive hashes
distinguish them. The previous artifact is the actual preserved prior harness
package, not a rebuilt old version label.

The executable was built before the final source commit; its embedded commit
identifies the checkout base and records the dirty tree. BUILD.json pins every
compiled source input and the toolchain. A later CI or PC rebuild records its
own commit and binary identity and is a different executable.

This directory stays outside the package payload so recording its hash does not
change the archive being measured. CONTENTS.json describes the measured payload;
SHA256.json pins this receipt collection. Physical GTX 1070 execution, GPU memory
and human novice usability remain separate PC checks.
