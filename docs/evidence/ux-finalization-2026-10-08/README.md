# UX finalization evidence

This is a new evidence set for the October 8 UX pass. Historical final-pass
evidence is unchanged. The exact build inputs and locally frozen binary identity
are in `build-info.json` and `source-validation.json`.

- Full Rust run: **183 passed, zero failed, one ignored**. The ignored existing
  longevity test was already executed in the earlier final-pass evidence; it was
  not rerun for the UX pass. Format and strict all-target Clippy passed.
- Installer: **12 passed**, including a real no-subcommand `alt` launch through
  PATH and terminal restoration. Includes ten interrupted-update fault points,
  rollback, integrity rejection, shadowing commands and quoted custom paths.
- Product walkthrough: **109 asserted checks** at 120×40, 80×24 and 60×18.
  It visits every page by keyboard and mouse and every Settings entry, tests
  setup errors and preserved forms, scrolling, action scope, resize, normal exit
  and terminal restoration after a real startup storage failure.
- The broader terminal suite's initial receipt has **11 passing groups and one
  failed group**. The Task test's literal phrase crossed a content-panel line
  after the sidebar widened. The helper now joins displayed panel words; all
  existing phrase and database assertions remain. The Task rerun passed.
  Final first-run and Task receipts identify each frozen binary separately.
- The broader suite includes drafts, recovery, project files, jobs, verification,
  practice, three harness sizes, real-Goose allowances, and 30 input-pressure
  journeys. These fixture passes do not measure model reasoning.

## Failures found and retained

`baseline-product.log` demonstrates the original globally offered connection-form
action failing outside its form. `product.log` records the clipped Cancel label
at 60×18. Both product defects were fixed and the complete journey rerun.

`rust-hint-test-initial.log` is an initial regression test that asserted a dialog
before its asynchronous brief read completed; the corrected test awaits the
actual result and checks the same dialog. `rust-footer-fit-initial.log` and
`rust-footer-fit-second.log` retain failed attempts to fit the Quit hint. The
final rendering assertion requires the complete hint at the minimum width.

Earlier monochrome and small-terminal observations are under `before/`. Final
screenshots are actual observed terminal buffers, rendered to PNG with pyte and
Pillow. A real SQLite exclusive lock exposes the startup frame for capture; the
test releases it before continuing and verifies that Home opens. No startup
delay or fabricated progress is added to the application.

Exact receipts distinguish builds used before the last two mouse fixes and
before the final footer shortening. The final product run uses a separately
copied immutable binary, so subsequent builds cannot change its identity.

No live model inference, model download, training, GPU qualification or human
novice session is counted in this evidence. The previous matched model pilot
remains 1/4 baseline and 1/4 candidate.
