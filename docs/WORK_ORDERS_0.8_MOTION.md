# 0.8 motion work orders (M01–M05)

Goal: an interface that moves like a high-end web app without ever slowing
input, hiding state or costing an idle terminal anything. Built on the 0.7
branch (`cloud/0.7-2026-10-11`); fixtures only, no new dependencies. Details
and measurements are in [MOTION.md](MOTION.md).

| Order | Scope | Acceptance |
|---|---|---|
| M01 | Motion engine: clock, easing, tweens, colour blend; Full/Reduced/Off saved and in Settings + palette; `ALT_MOTION`; adaptive 16/60 ms frame pacing; clock-driven spinners; gliding meters | Unit tests per primitive; settings/palette reach; pacing test |
| M02 | Page cascade; dialog dim, pop, shadow and fade-out; quantised stages for bandwidth | Final frame exact; Reduced in place; Off in place; pressure smoke 30/30 |
| M03 | Toasts equal to the status line; streaming reveal with cursor; tool flashes and spinners; shimmer | Toasts never stale; progress never toasts; history whole |
| M04 | Gradient wordmark; focus breathing only while busy; hover and click feedback; launch splash that yields to keys | Idle frames identical; keys pass through splash; Reduced has no splash |
| M05 | Proof: frame-time budget test (Off vs Full), PTY bandwidth/idle check, GIF clips and stills, docs | Release p95 < 8 ms; ≤ ~4x bytes of Off; idle equal to Off |

## Status (2026-10-11)

| Order | Commit | Test | Result | Deferred |
|---|---|---|---|---|
| M01 | f861ba8 | motion unit tests, `motion_is_a_saved_setting…`, `adaptive_pacing…`, meters | Pass | — |
| M02 | 554563c | `pages_slide_in…`, `dialogs_pop_in…`, fx tests, all TUI smokes | Pass; also fixed pressure-smoke stalls by quantising transitions | — |
| M03 | 6d78a2e | toast, reveal, flash tests; all TUI smokes | Pass; `smoke_tui` drains its PTY between processes | — |
| M04 | b70f236 | splash, hover/press, breathing/gradient tests; all TUI smokes | Pass | — |
| M05 | this PR's last commit | `frame_time_stays_inside_budget…`, `capture_motion.py --check` | Pass: release 1.44 ms median, 2.45 ms p95 at 160×48; idle bytes equal to Off | Reference-PC frame time and perceived smoothness; non-xterm terminals |
