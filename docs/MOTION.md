# Motion

Alt animates its interface the way a modern web app does: pages cascade in,
dialogs pop open over a dimmed page, notices float in and leave, and replies
arrive with a soft cursor. Motion is never a reason to wait. Keys act the moment
they arrive. No text is held back for longer than a quarter of a second, and
with motion **Off** every frame is drawn in its final state.

![Launch](screenshots/0.8/motion/launch-120x40.gif)

## Choosing how much moves

Open **Settings → Appearance → Motion**, or press **Ctrl+P** and type `motion`.

| Level | What you see |
|---|---|
| **Full** (default) | Page cascade, dialog pop with overshoot, toasts that slide, gliding meters, focus glow while a task runs, hover and click feedback, a 0.7 s launch splash |
| **Reduced** | Fades and colour changes only. Nothing slides, scales or breathes, and there is no splash |
| **Off** | No animation. Toasts are not shown; the status line still carries every notice |

`ALT_MOTION=full|reduced|off` overrides the saved choice for one launch.
`NO_MOTION` (any value), `TERM=linux` (the Linux console) and `TERM=dumb` always
turn motion off.

## What moves, and for how long

| Effect | Where | Duration | Levels |
|---|---|---|---|
| Page cascade: rows settle top to bottom, each sliding a few columns and fading up | Content area on every page change | 190 ms | Full (Reduced: fade only) |
| Dialog pop: backdrop dims, dialog grows from 86% with a slight overshoot, drop shadow | Every dialog and approval | 170 ms in, 110 ms out | Full (Reduced: fade only) |
| Toast: the current notice floats in at the top right with a ✓ ✕ • glyph | Page area; never takes clicks | 3.2 s | Full, Reduced |
| Streaming reveal with a `▍` cursor | The reply being generated | catches up at ≥120 chars/s, at most ~250 ms behind | Full, Reduced |
| Tool status flash; running tools spin | Activity column | 650 ms | Full, Reduced |
| Gliding meters (eighth-cell steps) | Context meter, job progress | 420 ms | Full |
| Shimmer on progress of unknown size | Job bar | continuous while the job runs | Full |
| Focus ring breathing | Focused panel border | 2.6 s period, only while a task runs | Full |
| Hover glow and click flash | Any button, row, tab or card | 120 ms / 140 ms | Full, Reduced |
| Launch splash | Whole screen; any key passes through and ends it | 720 ms | Full |

A toast always shows the current status line text. A newer notice replaces it,
and an error or progress update retires it at once, so a toast never shows
anything stale. Identical repeated notices collapse into one card with a count
(×2).

## How it is built

- `src/tui/motion.rs`: one clock (pinnable in tests), easing curves (ease-out
  cubic, ease-in-out cubic, ease-out-back), keyed tweens, RGB blending, and the
  state of transitions, toasts, the reveal, flashes, hover and the splash.
  Every animation is a pure function of time.
- `src/tui/fx.rs`: effects applied to the frame buffer after widgets draw
  (fade, slide, clipped reveal, shadow, fade-out, gradient, tint). They only
  recolour or move cells that widgets already drew. Layout, hit targets and text
  are exactly what the widgets produced, so a click during a transition still
  lands on the target it will settle on.
- **Whole frames:** each frame is written once through a buffered backend
  inside a synchronized update, so a full-screen effect is never seen half
  drawn.
- **Adaptive frame pacing:** Alt draws every 16 ms while something animates and
  every 60 ms otherwise, and any input or update is drawn within one fast frame.
  Spinners follow the clock, so their speed doesn't depend on the frame rate.

## Budgets and how they are checked

**Frame time.** `frame_time_stays_inside_budget_with_every_effect_active` runs
the same scripted session (page changes, dialogs opening and closing, toasts, a
gliding meter, a busy task, 40 messages) at 160×48 with motion Off and Full.
Measured in a cloud container (release build, not the reference GTX 1070 PC):

| Build | Off median / p95 | Full median / p95 |
|---|---|---|
| release | 1.39 / 4.17 ms | 1.44 / 2.45 ms |
| debug | 13.8 / 20.5 ms | 14.3 / 23.6 ms |

The test fails above 8 ms p95 in release (60 ms in debug), or at
`ALT_FRAME_BUDGET_MS`.

**Bandwidth.** Effects that cover an area repaint every cell they touch. Each
transition is therefore drawn in a few distinct stages, and identical frames
diff to nothing:

- the page cascade passes each row through two stages;
- a dialog pops in three stages;
- the backdrop dims in one step.

`python3 scripts/capture_motion.py --check` runs real sessions through a PTY and
fails if motion writes more than about 4× the bytes of the same session with
motion off, or if a settled screen keeps writing. Bytes written per session
(including the launch splash), from `screenshots/0.8/motion/clips.json`:

| Session | 80×24 Full / Off | 120×40 Full / Off |
|---|---|---|
| Launch | 17 KB / 6 KB | 46 KB / 17 KB |
| Four page changes | 58 KB / 19 KB | 142 KB / 51 KB |
| Palette open, type, close | 54 KB / 22 KB | 110 KB / 54 KB |
| Two theme changes with toasts | 157 KB / 46 KB | 248 KB / 95 KB |
| Idle, settled (1.5 s) | 0.6 KB / 0.6 KB | 0.6 KB / 0.6 KB |

An idle Alt writes the same with motion on as off: focus rings only breathe
while a task runs.

**Behaviour.** Every TUI smoke script runs with motion at its default (Full).
They include 30 pressure journeys under lock contention, real-Goose allowance
journeys, workbench, harness, verification and practice. Unit tests cover each
effect's timing, the exact final frame, Reduced keeping text in place, and Off
drawing in place.

## Clips and stills

Recorded with `python3 scripts/capture_motion.py`, which replays every byte Alt
wrote at its arrival time into a terminal emulator:

- [Launch, 80×24](screenshots/0.8/motion/launch-80x24.gif) ·
  [page changes](screenshots/0.8/motion/pages-80x24.gif) ·
  [palette](screenshots/0.8/motion/palette-80x24.gif) ·
  [theme change and toast](screenshots/0.8/motion/theme-80x24.gif)
- Stills at 120×40: [splash](screenshots/0.8/motion/launch-splash-120x40.png),
  [cascade](screenshots/0.8/motion/launch-cascade-120x40.png),
  [settled](screenshots/0.8/motion/launch-settled-120x40.png),
  [dialog pop](screenshots/0.8/motion/palette-dialog-pop-120x40.png),
  [dialog open](screenshots/0.8/motion/palette-dialog-open-120x40.png),
  [toast](screenshots/0.8/motion/theme-toast-120x40.png)

Not measured here: frame time and perceived smoothness on the reference PC, and
terminals other than xterm-compatible emulators replayed through pyte.
