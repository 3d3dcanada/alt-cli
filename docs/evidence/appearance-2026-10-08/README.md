# Appearance pass — 2026-10-08

Implemented six full-interface themes, a custom RGB accent, four saved layout
choices, optional decorative graphics, a card-based wide Home screen and visible
mouse navigation in Focus. The user guide and actual image gallery are in
[Appearance](../../APPEARANCE.md).

## Completed checks

| Check | Result |
|---|---|
| Rust unit and integration suites | 191 passed, 0 failed, 1 existing opt-in longevity test ignored |
| Formatting and strict all-target Clippy | Passed |
| Appearance PTY journeys | 21 groups passed; 198 page visits across six themes and four layouts |
| General product PTY journey | 112 checks passed across all 11 pages and 18 Settings entries |
| Existing TUI regression suites | All eight passed; initial test timing failure retained and corrected |
| Portable Linux package gates | All eight passed, including 14 offline image references and actual beta 3 upgrade/rollback |
| Installed package appearance | Aurora and Focus saved, rendered correctly and survived restart |

The appearance matrix exercises 120×40, 80×24 and 60×18 layouts, actual resize
events, mouse-only Focus navigation, dashboard arrow/mouse actions, all six
permission dialogs, theme persistence and saved-theme startup. It checks that
ordinary typing cannot approve a tool and that clicking Reject rejects it. It
also covers graphics off, draft preservation across settings and restart, invalid
accent recovery, custom accent saving, Reset and terminal restoration. Every
visited page's alphanumeric text is checked against the actual emitted colors at
4.5:1 contrast. Palette unit tests cover the semantic colors and 750 custom-accent
combinations, including black and white extremes.

The eight existing suites cover first-run setup, Task/check/memory/access, files
and jobs, independent verification, draft recovery, state recovery, practice
projects and allowance controls. All use deterministic local fixtures; allowance
also uses the already installed real Goose engine. No model weights were loaded,
downloaded, trained or evaluated in this appearance pass.

## Exact build and retained evidence

`BUILD-debug.json` records the tested source inputs. `VALIDATION.json` records
the frozen binary hash and test totals. Its source fingerprint is
`73b7aff5c176f9627e61553c42eceb6efe0d3f6d03e07f7990db0af067510611`.
Both terminal suites verify that the binary hash remained unchanged through all
restarts. The debug build predates the commit, so its embedded Git label says
modified; source inputs identify the exact implementation independently.

The `appearance/` and `product/` directories retain all observed text buffers and
journey receipts. The appearance receipt records hashes for the complete set of
locally captured PNGs; the ten selected PNGs shipped in the repository and offline
package are in [screenshots/appearance](../../screenshots/appearance/), with text
buffers and a separate `CAPTURE.json`. Other PNGs remain in the cloud capture
directory named by the run; their text versions are retained here.

`regressions/summary.json` preserves the first run's seven passes and one timing
failure. After renaming a conversation with a search filter active, the test
cleared that filter and immediately requested export before refreshed rows
arrived. The unchanged test passed standalone. The fix waits for the actual
rename-complete message and restored list row; all export assertions remain.
Three concurrent reruns passed on the unchanged application binary.
`regressions/final-summary.json` records the final eight-suite result.

Earlier preflight work corrected test selectors for the actual Reject button
and appended Appearance setting. Visual review shortened clipped dashboard copy;
independent source review changed appearance menus from Cancel to Close because
choices save immediately. The final receipts are from the resulting build.

## Reproduction and scope

Build with the repository's pinned toolchain, then run:

```bash
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
python3 scripts/smoke_appearance_tui.py
python3 scripts/smoke_product_tui.py
```

Set `ALT_TEST_BINARY` to a frozen binary and `ALT_TUI_SCREENSHOTS` to an output
directory to retain screenshots and receipts. Cloud Python tools are in
`/workspace/.alt-tools/python`. The new appearance journey is included in
`scripts/setup-cloud.sh` and therefore in GitHub Verify.

This validates the listed interactions and emitted colors. It does not establish
model reasoning quality, every terminal emulator/font combination, physical
GTX 1070 performance or uncoached novice acceptance. Those gates remain open.
Earlier evidence directories remain unchanged.

## Portable package and GitHub

The clean portable build is source commit
`43df077a5d50afdd87fea3e106dc2bc5ab70e5c4`. Its application source fingerprint
matches the frozen debug binary used by the full matrix. The exact package passed
all eight gates, including installed operation on Debian 11 and Ubuntu 24.04,
actual published beta 3 upgrade/rollback/re-upgrade, research archive integrity,
SBOM validation and 14 offline image references checked against manifest hashes.
The two curated screenshot folders remain bounded in the core offline package;
historical evidence stays in the separately verified research archive.

A further actual PTY check installed this optimized package through `install.sh`,
selected Aurora and Focus in the public appearance controls, then reopened the
same state. Both settings and observed colors persisted; terminal attributes
restored on both exits. The receipt, reproduction script, observed text and exact
build identities are retained in `portable/`.

- Portable binary SHA-256:
  `48f7d59efc2e409a00d5f7095e63e38107de7105c35c0de845ed1e4df3882891`.
- Archive SHA-256:
  `9094b5ab4853fe1876ca51f840f8e1848966dd1af20db2db7123bc546dc9f4ab`.
- No release was published. The archived package's documentation/evidence
  corresponds to the source commit above; this later evidence-only commit records
  its completed tests without changing application or packaging code.

[GitHub Verify](https://github.com/3d3dcanada/alt-cli/actions/runs/37777896752)
was still running when the final evidence snapshot was saved. Its JSON snapshot
is retained in `portable/github-verify-status.json`; local passes are not a claim
of completed remote CI. The duplicate push run was cancelled by the workflow's
normal branch concurrency setting.

The latest GitGuardian check still reports earlier incident **38001251** for a
public log checksum in `final-pass-2026-10-08/protocols/SHA256.json`. Its full
response is retained in `portable/gitguardian-check.json`. No new finding is
reported for this styling pass. The checksum evidence remains intact; external
false-positive dismissal is still outstanding.
