# Uncoached novice acceptance

Automation cannot establish that a person understands the interface. Run this
protocol with at least five consenting participants unfamiliar with Alt, using
copies of sample projects and no private credentials. Do not coach or enter
commands for the participant. Say the task, observe, and record confusion before
explaining any recovery. Record aliases only; do not record prompts or screens
containing private data without consent.

`python3 scripts/novice_session.py --output session.json` asks for observed results.
`--template` creates an explicitly unperformed worksheet. It cannot count as a pass.
Tasks cover connection, folder selection, draft recovery, verification coverage,
undo, backup/restore, access choices, and cancellation. Repeat at 80×24 and at the
participant's normal size; keyboard-only navigation must remain usable.

Acceptance requires at least four of five people to finish each core task without
coaching, all drafts retained, no incorrect belief that a build or empty test run
proves behavior, and successful recovery from connection and command failures.
Document exact Alt binary/source identity, model/runtime, hardware, terminal and
all failed journeys. Resolve confusing wording and repeat affected tasks with new
participants. These thresholds are release gates, not a statistical claim of
population-wide usability.

The automated `smoke_pressure_tui.py` journeys separately measure input visibility
and cancellation under project-lock pressure at 120×40, 80×24, and 60×18. A 500 ms
95th-percentile input target and 1 s cancellation target are engineering goals.
Raw screen failures remain in the report. Automated passes never count as human
sessions. No human study has been performed in this cloud environment.
