# Neon interface work orders (v0.7)

Owner request 2026-10-10: the TUI must always be usable at launch, the model
picker must describe every choice distinctly, everything must be changeable
inside the interface (mouse and keyboard), skills/tools must have explicit
on/off controls, and the look advances to a selectable neon theme family with
proper layout modes. No model weights, provider behavior or access-policy
semantics change in this set.

| Order | Deliverable | Acceptance |
| --- | --- | --- |
| N01 | Model picker shows distinct, described rows | Every model row shows name plus size, parameters, quantization and modified date where the server provides them; rows on the same page never render with identical descriptions; unknown metadata is labelled instead of hidden; existing wizard strings and `smoke_tui.py` still pass |
| N02 | Launch is never a dead end | A missing or deleted project path is detected at load and the first screen offers recent projects, the practice sandbox, opening a folder and creating a folder; dismissing the chooser opens the practice sandbox so every launch lands in a working workspace; recent projects are recorded and offered |
| N03 | Selectable theme engine | A theme registry with at least five complete themes (default neon, amber CRT, IRIS, bleached, stock) covering every interface color, border style and glyph; choice persists in preferences and changes live without a restart; stock theme reproduces the previous palette |
| N04 | Neon presentation pass | Focused panels, status meters for context budget and allowances, connection LED, animated busy indicator, message role badges, tool-status cards and colored diffs all drawn through the theme; minimum 80×24 remains fully usable and 60×18 keeps its notice |
| N05 | Settings organized as tabs | Settings groups into Appearance / Models / Access / Tools / Runtime / Data tabs; every previously reachable setting stays reachable from the keyboard in fewer or equal keystrokes |
| N06 | Command palette search | Ctrl+P opens a fuzzy-filterable palette containing every page, setting and manager action; typing filters, arrows move, Enter runs; mouse selection works |
| N07 | Skills and tools manager | One manager lists bundled skills, packs and extension tools with description, prerequisites and a persisted enabled/disabled toggle; disabled entries are excluded from what the model is offered; nothing auto-enables |
| N08 | Layout modes | Sidebar, top tabs and bottom tab layouts plus a wide split-pane mode (main page plus one secondary pane); Ctrl+L cycles; the choice persists; every mode keeps the full keyboard map |
| N09 | Verification and evidence | fmt, strict clippy, `cargo test`, `smoke_tui.py`, `smoke_task_tui.py` and screenshots at 80×24 and a wide size recorded; CHANGELOG and this file updated with outcomes, including anything deferred |

Out of scope by owner instruction: no new dependencies, no provider/model
behavior changes, no access-policy weakening, no auto-enabling of tools.

## Status 2026-10-10

All nine orders delivered in the working tree (not yet committed). Evidence:

| Order | Outcome |
| --- | --- |
| N01 | `models::ServerModel` carries server metadata; every model row and the wizard picker show distinct per-model descriptions plus a family blurb; `smoke_tui.py` wizard strings unchanged and passing |
| N02 | Dead project detected at load → chooser (current folder, 5 recents, browse, create, practice); Esc opens `data/sandbox` with a README; covered by `a_dead_saved_project_opens_a_chooser_and_esc_lands_in_the_sandbox` |
| N03 | `tui/theme.rs`: five complete themes, persisted `preferences.theme`, live redraw; `themes_have_unique_names_and_lookup_falls_back_to_default` |
| N04 | Header LED/access badge/spinner/context meter, role badges, tool cards, themed panels and nav pointer; 80×24 and 60×18 behavior unchanged (smoke resize step passes) |
| N05 | Five settings tabs; `settings_tabs_cover_every_row_exactly_once` proves the full mapping |
| N06 | Fuzzy palette dialog (contains → subsequence scoring), keyboard + click; `palette_filter_matches_words_and_runs_pages` |
| N07 | `skills-manager` toggles persist in `preferences.skills_disabled`; toolbox list/read/helper all filter; active procedure cleared if disabled; no auto-enables |
| N08 | Sidebar / TopTabs / BottomTabs / Split (+live activity column ≥100 cols); Ctrl+L cycle persists `preferences.layout` |
| N09 | fmt + strict Clippy clean; serial `cargo test` green; `smoke_tui.py` 4/4 passing, `smoke_task_tui.py` passing; screenshots in `docs/screenshots/neon/` (80×24 smoke screens + 120×40 settings/palette/theme captures) |

Known environment flake, recorded honestly: `conversation_database_contention`,
`pinning_a_requirement`, `task_check_choices` (5-second job-wait budgets) and
one `smoke_tui.py` engine-spawn window fail intermittently when host load
average exceeds ~10 on this shared machine. Reproduced on the pristine
baseline commit under the same load; all pass serially or on retry. Not caused
by this change set.

