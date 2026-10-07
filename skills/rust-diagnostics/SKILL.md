---
name: rust-diagnostics
version: 1
license: Apache-2.0
prerequisites: cargo and the project's pinned Rust toolchain
helper: build
---
Inspect Cargo.toml, rust-toolchain.toml and the relevant module. Use the pinned
toolchain and project lockfile. Reproduce the failure with the configured check;
compiler success alone is distinct from behavioral acceptance.

Locate the first actionable compiler diagnostic. Quote its code, source location
and relevant note from actual output. Check ownership, lifetime, trait or type
requirements in the current source. Separate this observation from your proposed
cause. Consult the API for the installed crate version before inventing methods.

Patch one bounded symbol or range using its current handle. Preserve public APIs
and tests unless the user requested a migration. Avoid broad rewrites to bypass a
specific failure. Reread on stale handles and inspect syntax feedback. Run the
project's focused tests before broad checks, followed by its required formatting
and lint checks when appropriate. The build helper runs cargo test; custom flags,
workspace packages and offline dependency availability remain project-specific.

Check numeric boundaries, error paths and concurrent state where the contract
requires them. A build, zero-test runner or model-written success statement cannot
verify a behavioral requirement. Cite actual check evidence and source revision.

Recovery: use checkpoint undo for a failed implementation. For missing toolchains
or crates, report the actual missing prerequisite and use the project's setup
instructions. Repeating an unchanged failed command is useful only after the
relevant source or environment changed. Do not silently downgrade the toolchain
or alter dependency constraints just to obtain a green exit code.
