use super::{app::*, input::Editor};
use crate::{display_text, models::human_bytes};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap,
    },
};

use super::theme::Palette;
use crate::config::{Appearance, LayoutPreset};

struct Renderer {
    colors: Palette,
    decorations: bool,
}

impl Renderer {
    fn new(appearance: &Appearance) -> Self {
        let mut colors = Palette::for_preset(appearance.theme);
        if let Some((r, g, b)) = appearance.accent_rgb() {
            colors = colors.with_accent(Color::Rgb(r, g, b));
        }
        Self {
            colors,
            decorations: appearance.decorations,
        }
    }
    fn style(&self, color: Color) -> Style {
        Style::default().fg(color)
    }
    fn block(&self, title: impl Into<Line<'static>>, focused: bool) -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(if self.decorations {
                BorderType::Rounded
            } else {
                BorderType::Plain
            })
            .title(title)
            .title_style(self.style(self.colors.muted))
            .border_style(self.style(if focused {
                self.colors.accent
            } else {
                self.colors.border
            }))
            .style(Style::default().bg(self.colors.panel).fg(self.colors.text))
    }
    fn line(&self, text: impl Into<String>, color: Color) -> Line<'static> {
        Line::styled(display_text(&text.into()), self.style(color))
    }
    fn paragraph(&self, frame: &mut Frame, area: Rect, text: impl Into<Text<'static>>) {
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .style(self.style(self.colors.text)),
            area,
        );
    }
    fn inset(&self, area: Rect, x: u16, y: u16) -> Rect {
        area.inner(Margin {
            horizontal: x,
            vertical: y,
        })
    }

    fn button(
        &self,
        frame: &mut Frame,
        app: &mut App,
        area: Rect,
        label: &str,
        action: &'static str,
        active: bool,
    ) {
        frame.render_widget(
            Paragraph::new(format!(" {label} ")).style(
                Style::default()
                    .fg(if active {
                        self.colors.accent_text
                    } else {
                        self.colors.text
                    })
                    .bg(if active {
                        self.colors.accent
                    } else {
                        self.colors.select
                    }),
            ),
            area,
        );
        app.hits.push((area, Hit::Button(action)));
    }
    fn buttons(
        &self,
        frame: &mut Frame,
        app: &mut App,
        area: Rect,
        items: &[(&str, &'static str)],
    ) {
        let mut x = area.x;
        for (label, action) in items {
            let width = (label.chars().count() as u16 + 2).min(area.right().saturating_sub(x));
            if width < 3 {
                break;
            }
            let rect = Rect::new(x, area.y, width, 1);
            self.button(frame, app, rect, label, action, false);
            x = x.saturating_add(width + 1);
        }
    }

    fn startup(&self, frame: &mut Frame) {
        frame.render_widget(
            Block::default().style(Style::default().bg(self.colors.bg).fg(self.colors.text)),
            frame.area(),
        );
        let area = self.centered(frame.area(), 66, 12);
        frame.render_widget(
            Paragraph::new(vec![
                self.line("", self.colors.text),
                self.line(
                    if self.decorations {
                        "     ▄▀█  █   ▀█▀"
                    } else {
                        "     ALT"
                    },
                    self.colors.accent,
                ),
                self.line(
                    if self.decorations {
                        "     █▀█  █▄▄  █"
                    } else {
                        ""
                    },
                    self.colors.accent,
                ),
                self.line("", self.colors.text),
                self.line("     Your models. Your workspace.", self.colors.text),
                self.line("", self.colors.text),
                self.line(
                    "     Opening settings, conversations and saved drafts…",
                    self.colors.muted,
                ),
                self.line("", self.colors.text),
                self.line(
                    format!("     Alt {}", env!("CARGO_PKG_VERSION")),
                    self.colors.muted,
                ),
            ])
            .block(self.block(" Welcome to Alt ", true)),
            area,
        );
    }

    fn draw(&self, frame: &mut Frame, app: &mut App) {
        let area = frame.area();
        app.hits.clear();
        frame.render_widget(
            Block::default().style(Style::default().bg(self.colors.bg).fg(self.colors.text)),
            area,
        );
        if area.width < 60 || area.height < 18 {
            self.paragraph(
                frame,
                self.inset(area, 2, 1),
                vec![
                    self.line("ALT", self.colors.accent),
                    self.line(
                        "Make this terminal at least 60 columns × 18 rows.",
                        self.colors.text,
                    ),
                    self.line(
                        "Resize to continue. Ctrl+Q saves drafts and leaves Alt.",
                        self.colors.muted,
                    ),
                ],
            );
            return;
        }
        let root = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(if app.job.is_some() { 6 } else { 3 }),
        ])
        .split(area);
        let header = Layout::horizontal([Constraint::Length(27), Constraint::Min(10)])
            .split(self.inset(root[0], 2, 0));
        self.paragraph(
            frame,
            header[0],
            vec![
                Line::from(vec![
                    Span::styled(
                        "ALT",
                        self.style(self.colors.accent).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  /  YOUR WORKSPACE", self.style(self.colors.muted)),
                ]),
                self.line(app.page.name(), self.colors.text),
                self.line(
                    format!("{} · F1 Help", env!("CARGO_PKG_VERSION")),
                    self.colors.muted,
                ),
            ],
        );
        // A visible page menu keeps mouse navigation available in Focus layout.
        let page_label = format!(
            "{} {}",
            app.page.name(),
            if self.decorations { "▾" } else { "+" }
        );
        let page_control = Rect::new(
            header[0].x,
            header[0].y + 1,
            (page_label.chars().count() as u16).min(header[0].width),
            1,
        );
        frame.render_widget(
            Paragraph::new(page_label).style(
                Style::default()
                    .fg(self.colors.accent)
                    .bg(self.colors.select),
            ),
            page_control,
        );
        app.hits.push((page_control, Hit::Button("palette")));
        let model = app
            .session
            .as_ref()
            .filter(|_| app.page == Page::Chat)
            .map(|s| s.profile.model.clone())
            .or_else(|| app.current_profile().map(|(_, p)| p.model.clone()))
            .unwrap_or_else(|| "Choose a model to get started".into());
        frame.render_widget(
            Paragraph::new(vec![
                self.line(model, self.colors.accent),
                self.line(
                    if app.busy {
                        "Working · Esc to stop"
                    } else if app.connecting {
                        "Connecting…"
                    } else if app.connected {
                        "Connected · saved locally"
                    } else {
                        "Local-first · you choose the model"
                    },
                    self.colors.muted,
                ),
            ])
            .alignment(ratatui::layout::Alignment::Right),
            header[1],
        );
        let appearance_label = format!(
            "{}{} · {}",
            if self.decorations { "◈ " } else { "" },
            app.preferences.appearance.theme.label(),
            app.preferences.appearance.layout.label()
        );
        let appearance_width = (appearance_label.chars().count() as u16 + 2).min(header[1].width);
        self.button(
            frame,
            app,
            Rect::new(
                header[1].right().saturating_sub(appearance_width),
                header[1].y + 2,
                appearance_width,
                1,
            ),
            &appearance_label,
            "appearance",
            false,
        );
        let sidebar = match app.preferences.appearance.layout {
            LayoutPreset::Auto => area.width >= 85,
            LayoutPreset::Sidebar => area.width >= 80 && area.height >= 22,
            LayoutPreset::Tabs | LayoutPreset::Focus => false,
        };
        let content = if sidebar {
            let cols =
                Layout::horizontal([Constraint::Length(24), Constraint::Min(30)]).split(root[1]);
            self.nav(frame, app, cols[0]);
            self.inset(cols[1], 1, 0)
        } else if app.preferences.appearance.layout == LayoutPreset::Focus && !app.nav_focus {
            self.inset(root[1], 1, 0)
        } else {
            let parts =
                Layout::vertical([Constraint::Length(2), Constraint::Min(8)]).split(root[1]);
            let labels = [
                "Home", "Chat", "Models", "Links", "Saved", "Settings", "Help", "Task", "Files",
                "Jobs", "Memory",
            ];
            let visible = (area.width as usize / 10).clamp(3, 7);
            let active = Page::ALL.iter().position(|p| *p == app.page).unwrap_or(0);
            let focus = if app.nav_focus { app.nav_index } else { active };
            let start = focus
                .saturating_sub(visible / 2)
                .min(Page::ALL.len() - visible);
            let cols = Layout::horizontal(vec![Constraint::Fill(1); visible]).split(parts[0]);
            for (column, i) in (start..start + visible).enumerate() {
                let page = &Page::ALL[i];
                let focused = app.nav_focus && app.nav_index == i;
                frame.render_widget(
                    Paragraph::new(format!("{}{}", if focused { "› " } else { "" }, labels[i]))
                        .style(
                            Style::default()
                                .bg(if focused {
                                    self.colors.select
                                } else {
                                    self.colors.bg
                                })
                                .fg(if focused || *page == app.page {
                                    self.colors.accent
                                } else {
                                    self.colors.muted
                                }),
                        ),
                    cols[column],
                );
                app.hits.push((cols[column], Hit::Nav(*page)));
            }
            self.inset(parts[1], 1, 0)
        };
        match app.page {
            Page::Home => self.home(frame, app, content),
            Page::Chat => self.chat(frame, app, content),
            Page::Models => self.models(frame, app, content),
            Page::Connections => self.connections(frame, app, content),
            Page::Sessions => self.sessions(frame, app, content),
            Page::Settings => self.settings(frame, app, content),
            Page::Help => self.help(frame, content, app.help_scroll),
            Page::Task => self.task(frame, app, content),
            Page::Files => self.files(frame, app, content),
            Page::Jobs => self.jobs(frame, app, content),
            Page::Context => self.context(frame, app, content),
        }
        self.footer(frame, app, root[2]);
        if app.dialog.is_some() || !app.permissions.is_empty() {
            app.hits.clear();
            self.modal(frame, app, area);
        }
    }

    fn nav(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let inner = self.inset(area, 1, 0);
        frame.render_widget(
            Block::default()
                .borders(Borders::RIGHT)
                .border_style(self.style(self.colors.border)),
            area,
        );
        let spacing = if inner.height >= 30 { 2 } else { 1 };
        for (i, page) in Page::ALL.iter().enumerate() {
            let rect = Rect::new(
                inner.x,
                inner.y + i as u16 * spacing,
                inner.width.saturating_sub(1),
                spacing,
            );
            let active = *page == app.page;
            let focused = app.nav_focus && app.nav_index == i;
            frame.render_widget(
                Paragraph::new(format!(
                    "{}{}  {}",
                    if focused { "›" } else { " " },
                    if i < 9 {
                        (i + 1).to_string()
                    } else if i == 9 {
                        "0".into()
                    } else {
                        "·".into()
                    },
                    page.name()
                ))
                .style(
                    Style::default()
                        .bg(if active || focused {
                            self.colors.select
                        } else {
                            self.colors.bg
                        })
                        .fg(if active || focused {
                            self.colors.accent
                        } else {
                            self.colors.muted
                        }),
                ),
                rect,
            );
            app.hits.push((rect, Hit::Nav(*page)));
        }
        if inner.height > 22 {
            let start = inner.bottom().saturating_sub(5);
            self.paragraph(
                frame,
                Rect::new(inner.x, start, inner.width, 5),
                vec![
                    self.line("ON THIS COMPUTER", self.colors.muted),
                    self.line(
                        if app.hardware.ram == 0 {
                            "Detecting hardware…".into()
                        } else {
                            format!("{} RAM", human_bytes(app.hardware.ram))
                        },
                        self.colors.text,
                    ),
                    self.line(
                        format!("{} CPU threads", app.hardware.threads),
                        self.colors.text,
                    ),
                    self.line("Ctrl+P  Actions", self.colors.muted),
                    self.line("F1      Help", self.colors.muted),
                ],
            );
        }
    }

    fn footer(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let inner = self.inset(area, 2, 0);
        let status_y = if let Some(job) = &app.job {
            let ratio = if job.progress.total > 0 {
                job.progress.received as f64 / job.progress.total as f64
            } else {
                0.0
            };
            let label = if job.progress.total > 0 {
                format!(
                    "{} · {} / {} · {}s",
                    job.label,
                    human_bytes(job.progress.received),
                    human_bytes(job.progress.total),
                    job.started.elapsed().as_secs()
                )
            } else {
                format!(
                    "{} · {} · {}s",
                    job.label,
                    job.progress.stage,
                    job.started.elapsed().as_secs()
                )
            };
            let progress = Rect::new(inner.x, inner.y, inner.width.saturating_sub(10), 1);
            frame.render_widget(
                Gauge::default()
                    .ratio(ratio.clamp(0.0, 1.0))
                    .label(label)
                    .gauge_style(
                        Style::default()
                            .fg(self.colors.accent)
                            .bg(self.colors.select),
                    ),
                progress,
            );
            let stage = job.progress.stage.clone();
            self.button(
                frame,
                app,
                Rect::new(progress.right() + 1, inner.y, 9, 1),
                "Pause",
                "pause-job",
                false,
            );
            self.paragraph(
                frame,
                Rect::new(inner.x, inner.y + 1, inner.width, 1),
                self.line(stage, self.colors.muted),
            );
            inner.y + 3
        } else {
            inner.y
        };
        self.paragraph(
            frame,
            Rect::new(inner.x, status_y, inner.width, 1),
            self.line(
                app.status.clone(),
                if app.status_error {
                    self.colors.bad
                } else {
                    self.colors.text
                },
            ),
        );
        let hint = if app.nav_focus {
            "↑↓ Choose page   Enter Open   Tab Return"
        } else {
            match app.page {
                Page::Files => "↑↓ Select  Enter View  E Edit  S Find  G Line  / Filter",
                Page::Jobs => "Enter Attach  Ctrl+] Detach  N Start  S Stop  R Restart  L Logs",
                Page::Context => "I Understands · C Correct · P Pin · U Unpin · R Refresh",
                Page::Home => "↑↓ Move · Enter · Tab Pages · Ctrl+P Find · Ctrl+Q Quit",
                Page::Chat => {
                    "Enter Send · Esc Stop · Ctrl+P Actions · Ctrl+L Allowance · Ctrl+D Drafts"
                }
                Page::Models => {
                    "←→ Library / Server / Hub   ↑↓ Select   Enter Use/Open   R Refresh   I Import   S Search"
                }
                Page::Connections => {
                    "↑↓ Select   Enter Use   A Add   E Edit   T Test   Delete Remove"
                }
                Page::Sessions => {
                    "↑↓ Select   Enter Resume   / Search   R Rename   E Export   Delete Archive"
                }
                Page::Task => {
                    "R Checks · U Undo · C Configure · I Understands · F Recovered · E Evidence"
                }
                Page::Settings => {
                    "↑↓ Choose setting   Enter Change   Settings are saved automatically"
                }
                Page::Help => "Alt+1…0 Pages   Tab Navigation   Ctrl+P Actions   Ctrl+Q Quit",
            }
        };
        self.paragraph(
            frame,
            Rect::new(inner.x, status_y + 1, inner.width, 1),
            self.line(hint, self.colors.muted),
        );
    }

    fn home(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let dashboard = self.decorations && area.width >= 86 && area.height >= 25;
        let compact = area.height < 19;
        let tiny = area.height < 13;
        let parts = Layout::vertical([
            Constraint::Length(if dashboard {
                7
            } else if tiny {
                2
            } else if compact {
                4
            } else {
                5
            }),
            Constraint::Min(4),
            Constraint::Length(2),
        ])
        .split(area);
        let connection = app
            .current_profile()
            .map(|(name, p)| format!("Model: {} · {}", name, p.model))
            .unwrap_or_else(|| "Connect a model, or explore a practice project first.".into());
        let mut welcome = vec![Line::styled(
            "Your models. Your workspace.",
            self.style(self.colors.text).add_modifier(Modifier::BOLD),
        )];
        if !compact {
            welcome.push(self.line(
                "Describe your goal in ordinary language. Alt helps with the steps.",
                self.colors.muted,
            ));
        }
        if !tiny {
            welcome.push(self.line(connection, self.colors.accent));
        }
        welcome.push(self.line(
            format!("Project: {}", app.preferences.project.display()),
            self.colors.muted,
        ));
        if !tiny {
            welcome.push(self.line(
                if app.engine_ready() {
                    "Agent engine ready · conversations stay on this computer"
                } else {
                    "Agent engine missing · install it below when you are ready"
                },
                if app.engine_ready() {
                    self.colors.good
                } else {
                    self.colors.warn
                },
            ));
        }
        if dashboard {
            let rows =
                Layout::vertical([Constraint::Length(4), Constraint::Length(3)]).split(parts[0]);
            let brand =
                Layout::horizontal([Constraint::Length(14), Constraint::Min(20)]).split(rows[0]);
            self.paragraph(
                frame,
                self.inset(brand[0], 1, 0),
                vec![
                    self.line("▄▀█  █   ▀█▀", self.colors.accent),
                    self.line("█▀█  █▄▄  █", self.colors.accent),
                    self.line("WORKSPACE", self.colors.muted),
                ],
            );
            self.paragraph(
                frame,
                brand[1],
                vec![
                    Line::styled(
                        "Your models. Your workspace.",
                        self.style(self.colors.text).add_modifier(Modifier::BOLD),
                    ),
                    self.line(
                        "Describe a goal. Explore an idea. Make something work.",
                        self.colors.muted,
                    ),
                    self.line(
                        format!("Project: {}", app.preferences.project.display()),
                        self.colors.muted,
                    ),
                ],
            );
            let states = Layout::horizontal([Constraint::Fill(1); 3])
                .spacing(1)
                .split(rows[1]);
            let model = app
                .current_profile()
                .map(|(_, p)| p.model.clone())
                .unwrap_or_else(|| "Choose your first model".into());
            let project = app
                .preferences
                .project
                .file_name()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| app.preferences.project.display().to_string());
            for (index, (title, value, action)) in [
                (
                    " ◈ MODEL ",
                    model,
                    if app.current_profile().is_some() {
                        "page-models"
                    } else {
                        "connect"
                    },
                ),
                (" ▣ PROJECT ", project, "project"),
                (
                    " ◇ TOOLS ",
                    if app.engine_ready() {
                        "Agent engine ready".into()
                    } else {
                        "Set up the agent engine".into()
                    },
                    "page-settings",
                ),
            ]
            .into_iter()
            .enumerate()
            {
                frame.render_widget(
                    Paragraph::new(value)
                        .style(self.style(self.colors.accent))
                        .block(self.block(title, false)),
                    states[index],
                );
                app.hits.push((states[index], Hit::Button(action)));
            }
        } else {
            self.paragraph(frame, parts[0], welcome);
        }
        let labels = [
            (
                if app.config.profiles.is_empty() {
                    "Connect your first model"
                } else {
                    "Start a new conversation"
                },
                if app.config.profiles.is_empty() {
                    "Choose your model app or a GGUF file. Setup guides you."
                } else {
                    "Tell the assistant what you want to accomplish."
                },
            ),
            (
                "Try a practice project",
                "Learn repair, real checks and undo in a fresh example folder.",
            ),
            (
                "Choose a project folder",
                "Pick the files Alt should work with.",
            ),
            (
                "Prepare project checks",
                "Optional: choose how to verify changes to this project.",
            ),
            (
                "Choose a connection",
                "Use Ollama, LM Studio, a model file, or another API.",
            ),
            (
                "Start with a guided task",
                "Understand, build, troubleshoot, check, or review.",
            ),
            (
                if app.engine_ready() {
                    "Agent engine is installed"
                } else {
                    "Install the agent engine"
                },
                "A separate local component that gives your model tools.",
            ),
            (
                "Keep a project brief",
                "Save goals, decisions, and next steps for future messages.",
            ),
        ];
        let (choices, guidance) = if parts[1].height >= 22 {
            let areas =
                Layout::vertical([Constraint::Length(18), Constraint::Min(4)]).split(parts[1]);
            (areas[0], Some(areas[1]))
        } else {
            (parts[1], None)
        };
        if dashboard {
            let columns = Layout::horizontal([Constraint::Fill(1); 2])
                .spacing(1)
                .split(choices);
            let glyphs = ["◈", "◇", "▣", "✓", "⇄", "✦", "⚙", "≡"];
            for (column, area) in columns.iter().enumerate() {
                let cards = Layout::vertical([Constraint::Fill(1); 4]).split(*area);
                for (row, area) in cards.iter().enumerate() {
                    let index = column * 4 + row;
                    let selected = index == app.home_selected && !app.nav_focus;
                    let panel = self
                        .block(
                            format!(" {}  {} ", glyphs[index], labels[index].0),
                            selected,
                        )
                        .title_style(
                            self.style(if selected {
                                self.colors.accent
                            } else {
                                self.colors.text
                            })
                            .add_modifier(Modifier::BOLD),
                        )
                        .style(
                            Style::default()
                                .bg(if selected {
                                    self.colors.select
                                } else {
                                    self.colors.panel
                                })
                                .fg(self.colors.text),
                        );
                    let inner = panel.inner(*area);
                    frame.render_widget(panel, *area);
                    frame.render_widget(
                        Paragraph::new(labels[index].1)
                            .style(self.style(self.colors.muted))
                            .wrap(Wrap { trim: true }),
                        self.inset(inner, 1, 0),
                    );
                    app.hits.push((*area, Hit::Home(index)));
                }
            }
        } else {
            let row_height = if compact { 1 } else { 2 };
            let rows = labels
                .iter()
                .map(|(title, hint)| {
                    ListItem::new(if compact {
                        vec![self.line(*title, self.colors.text)]
                    } else {
                        vec![
                            self.line(*title, self.colors.text),
                            self.line(*hint, self.colors.muted),
                        ]
                    })
                })
                .collect::<Vec<_>>();
            let mut state = ListState::default().with_selected(Some(app.home_selected));
            frame.render_stateful_widget(
                List::new(rows)
                    .highlight_style(Style::default().bg(self.colors.select))
                    .highlight_symbol("› ")
                    .block(self.block(
                        format!(
                            " Start here · {} / {} ",
                            app.home_selected + 1,
                            labels.len()
                        ),
                        !app.nav_focus,
                    )),
                choices,
                &mut state,
            );
            let list = self.inset(choices, 1, 1);
            let offset = state.offset();
            for i in offset..labels.len() {
                let y = list.y + (i - offset) as u16 * row_height;
                if y >= list.bottom() {
                    break;
                }
                app.hits.push((
                    Rect::new(list.x, y, list.width, row_height.min(list.bottom() - y)),
                    Hit::Home(i),
                ));
            }
        }
        if let Some(guidance) = guidance {
            let cols = Layout::horizontal([Constraint::Percentage(52), Constraint::Percentage(48)])
                .split(guidance);
            frame.render_widget(
                Paragraph::new(vec![
                    self.line("1  Describe the result you want.", self.colors.text),
                    self.line("2  Review commands and file changes.", self.colors.text),
                    self.line("3  Check the results together.", self.colors.text),
                    self.line("", self.colors.text),
                    self.line(
                        "Esc stops work · Conversations stay saved",
                        self.colors.muted,
                    ),
                ])
                .wrap(Wrap { trim: false })
                .block(self.block(" How it works ", false)),
                cols[0],
            );
            let context = app
                .current_profile()
                .map(|(_, p)| p.context_tokens)
                .unwrap_or(app.preferences.context_tokens);
            frame.render_widget(
                Paragraph::new(vec![
                    self.line(
                        format!("Context budget: {}K tokens", context / 1024),
                        self.colors.accent,
                    ),
                    self.line("Start with a Q4 model and 4K–8K context.", self.colors.text),
                    self.line("", self.colors.text),
                    self.line("GPU server or bundled CPU runtime.", self.colors.muted),
                ])
                .wrap(Wrap { trim: false })
                .block(self.block(" For smaller computers ", false)),
                cols[1],
            );
        }
        let (next, _, _) = app.next_step();
        self.paragraph(
            frame,
            parts[2],
            vec![
                self.line(format!("Next: {next}"), self.colors.accent),
                self.line(
                    if compact {
                        labels[app.home_selected].1
                    } else {
                        "Ctrl+P → Continue next step · Saved drafts: Ctrl+D"
                    },
                    self.colors.muted,
                ),
            ],
        );
        app.hits.push((
            Rect::new(parts[2].x, parts[2].y, parts[2].width, 1),
            Hit::Button("next-step"),
        ));
        if compact && parts[2].height > 1 {
            app.hits.push((
                Rect::new(parts[2].x, parts[2].y + 1, parts[2].width, 1),
                Hit::Home(app.home_selected),
            ));
        }
    }

    fn chat(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let compact = area.height < 12;
        let rows = Layout::vertical([
            Constraint::Min(if compact { 3 } else { 5 }),
            Constraint::Length(if compact { 3 } else { 5 }),
            Constraint::Length(1),
        ])
        .split(area);
        let (conversation, activity) = if rows[0].width >= 84
            && app.preferences.appearance.layout != LayoutPreset::Focus
        {
            let c = Layout::horizontal([Constraint::Percentage(66), Constraint::Percentage(34)])
                .split(rows[0]);
            (c[0], Some(c[1]))
        } else if app.tools_focus {
            (Rect::default(), Some(rows[0]))
        } else {
            (rows[0], None)
        };
        if conversation.width > 0 {
            let mut lines = Vec::new();
            if app.messages.is_empty() {
                lines.extend([
                    self.line(
                        "What would you like to make or figure out?",
                        self.colors.text,
                    ),
                    self.line("", self.colors.text),
                    self.line("Examples", self.colors.accent),
                    self.line("• Explain how this project works.", self.colors.muted),
                    self.line(
                        "• Add a settings screen with a dark theme.",
                        self.colors.muted,
                    ),
                    self.line(
                        "• Help me understand and fix this error.",
                        self.colors.muted,
                    ),
                    self.line(
                        "• Run the checks and explain any failures.",
                        self.colors.muted,
                    ),
                    self.line("", self.colors.text),
                    self.line(
                        "Your first message connects the selected model.",
                        self.colors.muted,
                    ),
                    self.line(
                        "No model selected? Open Connections first.",
                        self.colors.muted,
                    ),
                    self.line(
                        "Ctrl+P searches every action · Ctrl+D restores drafts.",
                        self.colors.muted,
                    ),
                ]);
            }
            for message in &app.messages {
                lines.push(self.line("", self.colors.text));
                lines.push(Line::styled(
                    format!(" {} ", message.role),
                    self.style(if message.role == "You" {
                        self.colors.accent
                    } else if message.role == "Notice" {
                        self.colors.warn
                    } else {
                        self.colors.good
                    })
                    .add_modifier(Modifier::BOLD),
                ));
                let mut code = false;
                for text in message.text.lines() {
                    if text.starts_with("```") {
                        code = !code;
                        lines.push(self.line(text, self.colors.muted));
                        continue;
                    }
                    lines.push(Line::styled(
                        text.to_string(),
                        if code {
                            Style::default().bg(self.colors.select).fg(self.colors.text)
                        } else if text.starts_with('#') {
                            self.style(self.colors.accent)
                        } else {
                            self.style(self.colors.text)
                        },
                    ));
                }
            }
            if let Some(submission) = &app.pending_prompt {
                lines.push(self.line(
                    if app.send_pending_on_ready {
                        "Submitted while connecting · next message stays in the editor"
                    } else {
                        "Interrupted submission retained · Ctrl+D to review before retry"
                    },
                    self.colors.warn,
                ));
                lines.push(self.line(submission.text.clone(), self.colors.muted));
            }
            if app.busy {
                lines.push(self.line(
                    format!(
                    "Working{}  {}s · {}",
                    ".".repeat(
                        (app.turn_started
                            .map(|s| s.elapsed().as_millis() / 400)
                            .unwrap_or(0)
                            % 4) as usize
                    ),
                    app.turn_started.map(|s| s.elapsed().as_secs()).unwrap_or(0),
                    if !app.permissions.is_empty() {
                        "Waiting for your tool approval"
                    } else {
                        app.inference_status
                            .as_ref()
                            .map(|s| s.stage.label())
                            .unwrap_or("Preparing saved task and source context")
                    }
                ),
                    self.colors.warn,
                ));
            }
            if let Some(s) = &app.inference_status {
                lines.push(self.line(
                    format!(
                    "Calls left: {} · generated tokens left: {}",
                    s.remaining_requests
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "no limit".into()),
                    s.remaining_generated_tokens
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "no limit".into())
                ),
                    self.colors.muted,
                ));
                lines.push(self.line(
                    format!(
                    "Prompt: {} / {} · output reserve: {}",
                    s.input_tokens
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "unknown".into()),
                    s.input_allowance,
                    app.session
                        .as_ref()
                        .map(|session| session
                            .profile
                            .context_tokens
                            .saturating_sub(s.input_allowance))
                        .unwrap_or(s.output_allowance)
                ),
                    self.colors.muted,
                ));
            }
            let paragraph = Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(self.block(" Conversation ", !app.tools_focus && !app.nav_focus));
            let count = paragraph
                .line_count(conversation.width.saturating_sub(2))
                .saturating_sub(2)
                .min(u16::MAX as usize) as u16;
            let scroll = count
                .saturating_sub(conversation.height.saturating_sub(2))
                .saturating_sub(app.chat_scroll);
            frame.render_widget(paragraph.scroll((scroll, 0)), conversation);
        }
        if let Some(activity) = activity {
            let chunks = Layout::vertical([
                Constraint::Length(4),
                Constraint::Min(3),
                Constraint::Length(3),
            ])
            .split(activity);
            let context = match (app.context_used, app.context_size) {
                (Some(used), Some(size)) => format!("Context: {used} / {size} tokens"),
                _ => format!(
                    "Context: {} tokens configured",
                    app.session
                        .as_ref()
                        .map(|s| s.profile.context_tokens)
                        .or_else(|| app.current_profile().map(|(_, p)| p.context_tokens))
                        .unwrap_or(app.preferences.context_tokens)
                ),
            };
            self.paragraph(
                frame,
                self.inset(chunks[0], 1, 0),
                vec![
                    self.line("ACTIVITY & CONTEXT", self.colors.accent),
                    self.line(context, self.colors.muted),
                    self.line(
                        format!("Project brief: {} chars", app.brief.chars().count()),
                        self.colors.muted,
                    ),
                    self.line(
                        if app.trust_session {
                            "Tools trusted for this session"
                        } else {
                            "Tool actions request approval"
                        },
                        if app.trust_session {
                            self.colors.warn
                        } else {
                            self.colors.muted
                        },
                    ),
                ],
            );
            let items = app
                .tools
                .iter()
                .map(|tool| {
                    ListItem::new(vec![
                        self.line(tool.title.clone(), self.colors.text),
                        self.line(
                            format!("  {}", tool.status),
                            match tool.status.as_str() {
                                "completed" => self.colors.good,
                                "failed" => self.colors.bad,
                                _ => self.colors.warn,
                            },
                        ),
                    ])
                })
                .collect::<Vec<_>>();
            if items.is_empty() {
                frame.render_widget(
                Paragraph::new(
                    "No tool actions yet.\n\nCommands, file changes, and results will appear here.",
                )
                .wrap(Wrap { trim: false })
                .block(self.block(" Tools ", app.tools_focus)),
                chunks[1],
            );
            } else {
                let mut state = ListState::default().with_selected(Some(app.tool_selected));
                frame.render_stateful_widget(
                    List::new(items)
                        .highlight_style(Style::default().bg(self.colors.select))
                        .block(self.block(" Tools · Enter for evidence ", app.tools_focus)),
                    chunks[1],
                    &mut state,
                );
                let inner = self.inset(chunks[1], 1, 1);
                let offset = state.offset();
                for i in offset..app.tools.len() {
                    let y = inner.y + (i - offset) as u16 * 2;
                    if y >= inner.bottom() {
                        break;
                    }
                    app.hits.push((
                        Rect::new(inner.x, y, inner.width, 2.min(inner.bottom() - y)),
                        Hit::Row(i),
                    ));
                }
            }
            self.buttons(
                frame,
                app,
                self.inset(chunks[2], 1, 1),
                &[("Evidence", "tool-details"), ("Task", "task")],
            );
        }
        let composer_block = self.block(
            if app.busy {
                if app.draft_saved {
                    " Next message · saved locally "
                } else {
                    " Next message · saving draft… "
                }
            } else if app.draft_saved {
                " Describe what you want · draft saved "
            } else {
                " Describe what you want · saving draft… "
            },
            !app.tools_focus && !app.nav_focus,
        );
        frame.render_widget(composer_block, rows[1]);
        let input = self.inset(rows[1], 1, 1);
        self.editor(
            frame,
            input,
            &app.composer,
            true,
            !app.tools_focus && !app.nav_focus,
        );
        app.hits.push((rows[1], Hit::Composer));
        let choices = if app.busy || app.connecting {
            vec![("Stop", "stop"), ("Brief", "brief"), ("Export", "export")]
        } else {
            vec![
                ("Send", "send"),
                ("New", "new"),
                ("Reconnect", "reconnect"),
                ("Allowance", "allowance"),
                ("Task progress", "task"),
                ("Starting tasks", "templates"),
                ("Export", "export"),
            ]
        };
        self.buttons(frame, app, rows[2], &choices);
    }

    fn selectable_list(
        &self,
        frame: &mut Frame,
        app: &mut App,
        area: Rect,
        title: &str,
        rows: Vec<(String, String)>,
        selected: usize,
    ) {
        let count = rows.len();
        let row_height = if area.height <= 3 { 1 } else { 2 };
        let mut state = ListState::default()
            .with_selected((count > 0).then_some(selected.min(count.saturating_sub(1))));
        let items = rows
            .into_iter()
            .map(|(title, hint)| {
                let mut lines = vec![self.line(title, self.colors.text)];
                if row_height > 1 {
                    lines.push(self.line(hint, self.colors.muted));
                }
                ListItem::new(lines)
            })
            .collect::<Vec<_>>();
        frame.render_stateful_widget(
            List::new(items)
                .block(self.block(format!(" {title} "), !app.nav_focus))
                .highlight_symbol("› ")
                .highlight_style(Style::default().bg(self.colors.select)),
            area,
            &mut state,
        );
        let inner = self.inset(area, 1, 1);
        let offset = state.offset();
        for i in offset..count {
            let y = inner.y + (i - offset) as u16 * row_height;
            if y >= inner.bottom() {
                break;
            }
            app.hits.push((
                Rect::new(inner.x, y, inner.width, row_height.min(inner.bottom() - y)),
                Hit::Row(i),
            ));
        }
    }

    fn models(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let rows = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);
        let tabs = Layout::horizontal([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(rows[0]);
        for (i, label) in [
            "1  On this computer",
            "2  On your server",
            "3  Hugging Face",
        ]
        .iter()
        .enumerate()
        {
            frame.render_widget(
                Paragraph::new(*label).style(
                    Style::default()
                        .fg(if app.model_tab == i {
                            self.colors.accent
                        } else {
                            self.colors.muted
                        })
                        .bg(if app.model_tab == i {
                            self.colors.select
                        } else {
                            self.colors.bg
                        }),
                ),
                tabs[i],
            );
            app.hits.push((tabs[i], Hit::ModelTab(i)));
        }
        let intro = match app.model_tab {
            0 => "Import a GGUF file or download one from Hugging Face. Files stay local.",
            1 => "Models your selected server already has. Refresh, then choose Use model.",
            _ => "Public GGUF models. Publisher labels do not guarantee capability or behavior.",
        };
        self.paragraph(frame, rows[1], self.line(intro, self.colors.muted));
        let items = match app.model_tab {
            0 => app
                .artifacts
                .iter()
                .map(|m| {
                    (
                        m.name.clone(),
                        format!(
                            "{} · {}",
                            human_bytes(m.bytes),
                            if m.source.is_some() {
                                "download checksum verified"
                            } else {
                                "imported file reference"
                            }
                        ),
                    )
                })
                .collect(),
            1 => app
                .server_models
                .iter()
                .map(|m| (m.clone(), "Available on the selected server".into()))
                .collect(),
            _ if app.hub_files.is_empty() => app
                .hub_models
                .iter()
                .map(|m| {
                    (
                        m.id.clone(),
                        format!("{} downloads · Enter to choose a file", m.downloads),
                    )
                })
                .collect(),
            _ => app
                .hub_files
                .iter()
                .map(|f| {
                    (
                        f.filename.clone(),
                        format!("{} · license: {}", human_bytes(f.bytes), f.license),
                    )
                })
                .collect(),
        };
        if app.model_count() == 0 {
            let text = match app.model_tab {
                0 => {
                    "Your model library is empty.\n\nChoose Import file if you already have a .gguf model. Or use the Hugging Face tab to find a downloadable model.\n\nYou can also connect Ollama or LM Studio without importing files."
                }
                1 => {
                    "No server models loaded yet.\n\nChoose a connection, then Refresh. If the server has no models, load one in your model application first."
                }
                _ => {
                    "Find a model by name, or open a publisher/model repository.\n\nTry Qwen3 heretic, Spark, or MiMo. Exact uncensored derivatives and runtime compatibility vary.\n\nSearch uses publisher labels; you can turn that filter off."
                }
            };
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(self.block(" Model library ", true)),
                rows[2],
            );
        } else {
            self.selectable_list(
                frame,
                app,
                rows[2],
                "Choose a model",
                items,
                app.model_selected,
            );
        }
        let details=match app.model_tab {
        0=>app.artifacts.get(app.model_selected).map(|m|format!("{}\nLicense: {}\n{} · CPU runtime {}",m.path.display(),m.license,if m.uncensored_claim{"Publisher labels this variant uncensored/abliterated"}else{"Uncensored status not established"},if app.runtime_ready(){"installed"}else{"not installed"})).unwrap_or_default(),
        1=>app.server_source.as_ref().and_then(|name|app.config.profiles.get(name).map(|p|(name,p))).map(|(name,p)|format!("Connection: {name}\nAddress: {}\nSelecting a model applies to new conversations.",p.endpoint)).unwrap_or_else(||"Open Connections to add a server, then Refresh.".into()),
        _=>{let filter=if app.variants_only{"Variant filter ON (publisher claims)"}else{"Variant filter OFF"};app.hub_files.get(app.model_selected).map(|f|format!("{}\n{} · download {}\nRunning a model needs additional memory for context and working buffers.",f.repo,filter,human_bytes(f.bytes))).unwrap_or_else(||format!("Search: {}\n{filter}\nOnly complete GGUF files with published SHA256 metadata are offered.",app.hub_query))},
    };
        self.paragraph(frame, rows[3], self.line(details, self.colors.muted));
        let buttons_list = match app.model_tab {
            0 => vec![
                ("Use model", "use-model"),
                ("Import file", "import"),
                ("Runtime", "runtime"),
                ("Refresh", "refresh-models"),
            ],
            1 => vec![
                ("Use model", "use-model"),
                ("Refresh", "refresh-models"),
                ("Connect", "connect"),
            ],
            _ => vec![
                ("Open / Download", "use-model"),
                ("Search", "hub-search"),
                ("Repository", "hub-repo"),
                ("Filter", "variant-filter"),
                ("Back", "hub-back"),
            ],
        };
        self.buttons(frame, app, rows[4], &buttons_list);
    }

    fn connections(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);
        self.paragraph(
            frame,
            rows[0],
            vec![
                self.line("Where your model runs", self.colors.text),
                self.line(
                    "Choose a saved connection, or add one with the guided setup.",
                    self.colors.muted,
                ),
            ],
        );
        let items = app
            .config
            .profiles
            .iter()
            .map(|(name, p)| {
                (
                    format!(
                        "{}{}",
                        if *name == app.config.default_profile {
                            "● "
                        } else {
                            "  "
                        },
                        name
                    ),
                    format!(
                        "{} · {}",
                        p.model,
                        if p.local_model.is_some() {
                            "local file"
                        } else if p.provider == crate::config::Provider::Ollama {
                            "Ollama"
                        } else {
                            "compatible API"
                        }
                    ),
                )
            })
            .collect();
        if app.config.profiles.is_empty() {
            frame.render_widget(Paragraph::new("No connections yet.\n\nChoose Add connection below. Alt will help you select your application, check its address, and choose a model.").wrap(Wrap{trim:false}).block(self.block(" Connections ",true)),rows[1]);
        } else {
            self.selectable_list(
                frame,
                app,
                rows[1],
                "Saved connections",
                items,
                app.connection_selected,
            );
        }
        if let Some((name, p)) = app.config.profiles.iter().nth(app.connection_selected) {
            self.paragraph(
                frame,
                rows[2],
                vec![
                    self.line(
                        format!(
                            "{name} · {} context tokens · {} steps per turn",
                            p.context_tokens, p.max_turns
                        ),
                        self.colors.text,
                    ),
                    self.line(
                        if p.local_model.is_some() {
                            "Alt loads this model locally in CPU mode.".into()
                        } else {
                            p.endpoint.clone()
                        },
                        self.colors.muted,
                    ),
                    self.line(
                        format!(
                            "Authentication: {}",
                            p.api_key_env.as_deref().unwrap_or("none configured")
                        ),
                        self.colors.muted,
                    ),
                ],
            );
        }
        self.buttons(
            frame,
            app,
            rows[3],
            &[
                ("Use", "use-connection"),
                ("Add connection", "connect"),
                ("Edit", "edit-connection"),
                ("Test", "test-connection"),
                ("Remove", "remove-connection"),
            ],
        );
    }

    fn sessions(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(area);
        frame.render_widget(
            self.block(
                if app.archived {
                    " Search archived conversations "
                } else {
                    " Search conversations · / to type "
                },
                app.search_focus,
            ),
            rows[0],
        );
        self.editor(
            frame,
            self.inset(rows[0], 1, 1),
            &app.session_search,
            false,
            app.search_focus,
        );
        app.hits.push((rows[0], Hit::Button("search-sessions")));
        let items = app
            .sessions
            .iter()
            .map(|s| {
                (
                    s.title.clone(),
                    format!("{} · {} · {}", s.updated, s.model, s.cwd),
                )
            })
            .collect();
        if app.sessions.is_empty() {
            frame.render_widget(Paragraph::new("No conversations match.\n\nStart a conversation from Home, clear the search, or switch between current and archived conversations.").wrap(Wrap{trim:false}).block(self.block(" Saved work ",true)),rows[1]);
        } else {
            self.selectable_list(
                frame,
                app,
                rows[1],
                "Saved work",
                items,
                app.session_selected,
            );
        }
        if let Some(s) = app.sessions.get(app.session_selected) {
            self.paragraph(
            frame,
            rows[2],
            self.line(
                format!(
                    "Project: {}\nResume uses the conversation's original model and connection.",
                    s.cwd
                ),
                self.colors.muted,
            ),
        );
        }
        self.buttons(
            frame,
            app,
            rows[3],
            &[
                ("Resume", "resume"),
                ("Rename", "rename"),
                ("Export", "export"),
                (if app.archived { "Restore" } else { "Archive" }, "archive"),
                (
                    if app.archived { "Current" } else { "Archived" },
                    "show-archived",
                ),
            ],
        );
    }

    fn settings(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(4),
        ])
        .split(area);
        self.paragraph(
            frame,
            rows[0],
            vec![
                self.line("Make Alt your own", self.colors.text),
                self.line(
                    "Enter changes a setting. Appearance updates immediately.",
                    self.colors.muted,
                ),
            ],
        );
        let context = app
            .current_profile()
            .map(|(_, p)| p.context_tokens)
            .unwrap_or(app.preferences.context_tokens);
        let budget = app
            .current_profile()
            .map(|(_, p)| p.max_turns)
            .unwrap_or(app.preferences.max_turns);
        let items = vec![
        (
            format!(
                "Context: {} tokens · {}",
                context,
                match context {
                    4096 => "Economy",
                    8192 => "Balanced",
                    16384 => "Extended",
                    _ => "Large",
                }
            ),
            "More context uses more memory. Cycle: 4K → 8K → 16K → 32K.".into(),
        ),
        (
            format!("Task budget: {budget} steps"),
            "Limits tool/model turns before asking you to continue. Cycle: 6 → 12 → 24.".into(),
        ),
        (
            format!(
                "Mouse: {}",
                if app.preferences.mouse {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            "Keyboard navigation is always available.".into(),
        ),
        (
            "Agent engine executable".into(),
            app.preferences
                .engine_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Auto-detect Goose; install from Home if missing".into()),
        ),
        (
            "Local model runtime executable".into(),
            app.preferences
                .runtime_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| {
                    "Auto-detect llama-server; install from Models if missing".into()
                }),
        ),
        (
            "Project brief".into(),
            "Edit the goals and decisions sent with each new message.".into(),
        ),
        (
            format!(
                "Access: {} · {} configured check(s)",
                app.preferences.access_policy.label(),
                app.task_view.checks.len()
            ),
            "Choose Review only, Guided changes, or Full access terminal.".into(),
        ),
        (
            "Fit Alt to this computer".into(),
            "Inspect hardware and choose a low-memory, balanced or larger context profile.".into(),
        ),
        (
            "Check this model's tool use".into(),
            "A short live probe of the exact selected uncensored model. Saves its evidence.".into(),
        ),
        (
            format!(
                "Selected model is uncensored/abliterated: {}",
                if app.current_profile().is_some_and(|(_, p)| p.uncensored) {
                    "claimed"
                } else {
                    "not marked"
                }
            ),
            "Enter records/removes your publisher claim, enabling the explicit live evaluation."
                .into(),
        ),
        (
            "Tools and workflows".into(),
            "Build, Git, HTTP, browser, security and selected external MCP tools.".into(),
        ),
        (
            "Storage and recovery".into(),
            "Backup, restore, disk usage, retention and redacted diagnostics.".into(),
        ),
        (
            "Model files and cache".into(),
            "Private Hub access, verified relocation and removing managed weights.".into(),
        ),
        (
            "Model and runtime settings".into(),
            "Output allocation, skills, workflow, effort and CPU/GPU settings.".into(),
        ),
        (
            "Benchmark selected model".into(),
            "Measure this computer; exports exact settings, timings and runtime memory.".into(),
        ),
        (format!("Tool focus: {}",app.preferences.tool_profile.label()),"Choose all tools, inspection, coding or terminal schemas; access mode stays your choice.".into()),
        ("Qualify model contexts and restart".into(),"Measure generation, memory, cancellation cleanup and restart for 2K, 4K and 8K.".into()),
        (format!("Appearance: {} · {}", app.preferences.appearance.theme.label(), app.preferences.appearance.layout.label()), "Six themes, your own accent color, layouts and decorative graphics.".into()),
    ];
        self.selectable_list(
            frame,
            app,
            rows[1],
            "Preferences",
            items,
            app.settings_selected,
        );
        self.paragraph(
            frame,
            rows[2],
            vec![
                self.line(
                    format!(
                        "Detected: {} RAM · {} available · {} CPU threads",
                        human_bytes(app.hardware.ram),
                        human_bytes(app.hardware.available_ram),
                        app.hardware.threads
                    ),
                    self.colors.muted,
                ),
                self.line(app.hardware.gpu.clone(), self.colors.muted),
                self.line(
                    "CPU is the default. GPU layers require a compatible llama-server build.",
                    self.colors.muted,
                ),
                self.line(format!("Data: {}", app.root.display()), self.colors.muted),
            ],
        );
    }

    fn help(&self, frame: &mut Frame, area: Rect, scroll: u16) {
        let text = vec![
        self.line("YOU DO NOT NEED TO KNOW HOW TO CODE", self.colors.accent),
        self.line(
            "Tell Alt the result you want. It can inspect your files, explain a plan, make changes, and run checks. Review what it did; models can make mistakes.",
            self.colors.text,
        ),
        self.line("", self.colors.text),
        self.line("1. CONNECT A MODEL", self.colors.accent),
        self.line(
            "A connection points to the application running your model. Use Ollama, LM Studio, another compatible server, or a local GGUF model file. The guided setup checks the address and lists models.",
            self.colors.text,
        ),
        self.line("", self.colors.text),
        self.line("2. CHOOSE A PROJECT", self.colors.accent),
        self.line(
            "Pick the folder containing the files you want to work on. The project brief keeps your goals and decisions handy. It is included with your messages.",
            self.colors.text,
        ),
        self.line("", self.colors.text),
        self.line("3. DESCRIBE THE JOB AND REVIEW ACTIONS", self.colors.accent),
        self.line(
            "Use ordinary language. Activity shows commands, edits, and actual results. Read each approval, then reject it, allow it once, or trust the session. Guided changes checkpoints edits and isolates configured checks. Full access enables arbitrary terminal commands with your OS access. Task progress shows diffs, undo and fresh check evidence.",
            self.colors.text,
        ),
        self.line("", self.colors.text),
        self.line("FIND YOUR WAY", self.colors.accent),
        self.line(
            "Click a page or press Alt+1…0. Tab focuses navigation. Ctrl+P searches all actions. Ctrl+L opens allowance; Ctrl+R reconnects; Ctrl+D restores drafts. Enter sends a message; Alt+Enter or Ctrl+J adds a new line. Esc stops a task. Ctrl+Q saves drafts and exits.",
            self.colors.text,
        ),
        self.line("", self.colors.text),
        self.line("MAKE IT YOUR OWN", self.colors.accent),
        self.line("Click the theme chip at the top right, or open Settings → Appearance. Choose from six themes, a custom accent, Automatic/Sidebar/Top tabs/Focus layouts, and decorative graphics. Changes apply immediately and are saved for next time. In Focus, press Tab to reveal navigation or click the page name at the top left to find a page. Ctrl+P always finds actions. Narrow windows adapt without changing your saved layout.", self.colors.text),
        self.line("", self.colors.text),
        self.line("COPY, PASTE AND MOUSE", self.colors.accent),
        self.line(
            "Paste with your terminal shortcut (usually Ctrl+Shift+V). To select text while mouse support is on, hold Shift while dragging, or turn Mouse off in Settings. Copy with your terminal shortcut (usually Ctrl+Shift+C). Ctrl+C stops work or exits. Mouse wheels scroll lists; keyboard arrows select items.",
            self.colors.text,
        ),
        self.line("", self.colors.text),
        self.line("YOUR CHOICE OF MODEL", self.colors.accent),
        self.line(
            "Alt works with compatible ordinary, uncensored and abliterated models. Your chosen server or GGUF file provides the model; the engine provides tools. Downloads and installs start when you choose them. Start with a modest context budget on an older computer.",
            self.colors.text,
        ),
        self.line("", self.colors.text),
        self.line("WHEN SOMETHING GOES WRONG", self.colors.accent),
        self.line(
            "Your conversation stays saved. Drafts autosave every 250 ms while storage responds, and flush on orderly exit. Abrupt termination can lose changes since the last completed save. Save errors are shown; retained submissions never resend automatically. Check the server, memory and tool evidence. Ctrl+O enters a model ID without inventory. Paused downloads resume when you choose the same file.",
            self.colors.text,
        ),
        self.line("", self.colors.text),
        self.line("ABOUT ALT", self.colors.accent),
        self.line(format!("Build: {}", crate::BUILD_LABEL), self.colors.text),
        self.line("github.com/3d3dcanada/alt-cli · Apache-2.0", self.colors.muted),
    ];
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0))
                .block(self.block(" A quick guide · PgUp/PgDn to scroll ", false)),
            area,
        );
    }

    fn centered(&self, area: Rect, width: u16, height: u16) -> Rect {
        let width = width.min(area.width.saturating_sub(4));
        let height = height.min(area.height.saturating_sub(2));
        Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        )
    }

    fn editor(
        &self,
        frame: &mut Frame,
        area: Rect,
        editor: &Editor,
        multiline: bool,
        focused: bool,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        if !multiline {
            let (text, column) = editor.line_view(area.width as usize);
            frame.render_widget(
                Paragraph::new(text).style(self.style(self.colors.text)),
                area,
            );
            if focused {
                frame.set_cursor_position((area.x + column.min(area.width - 1), area.y));
            }
        } else {
            use unicode_width::UnicodeWidthStr;
            let before = &editor.text[..editor.cursor];
            let row =
                before.lines().count().saturating_sub(1) + usize::from(before.ends_with('\n'));
            let row = if before.is_empty() { 0 } else { row };
            let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
            let column = before[line_start..].width();
            // Horizontal scrolling keeps the real cursor visible without wrapping
            // ambiguities; pasted multiline text retains its exact newlines.
            let top = row.saturating_sub(area.height.saturating_sub(1) as usize);
            let left = column.saturating_sub(area.width.saturating_sub(1) as usize);
            frame.render_widget(
                Paragraph::new(editor.text.clone())
                    .scroll((
                        top.min(u16::MAX as usize) as u16,
                        left.min(u16::MAX as usize) as u16,
                    ))
                    .style(self.style(self.colors.text)),
                area,
            );
            if focused {
                frame.set_cursor_position((
                    area.x + (column - left) as u16,
                    area.y + (row - top) as u16,
                ));
            }
        }
    }

    fn modal(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        if !app.permissions.is_empty() {
            self.permission_modal(frame, app, area);
            return;
        }
        let Some(dialog) = app.dialog.clone() else {
            return;
        };
        let rect = self.centered(
            area,
            84,
            match dialog {
                Dialog::Connection(_) => 20,
                Dialog::Confirm { .. } => 20,
                Dialog::Input {
                    multiline: true, ..
                } => 26,
                _ => 24,
            },
        );
        frame.render_widget(Clear, rect);
        match dialog {
            Dialog::Palette { query, selected } => {
                frame.render_widget(self.block(" Quick actions · type to search ", true), rect);
                let rows = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Min(4),
                    Constraint::Length(1),
                ])
                .split(self.inset(rect, 2, 1));
                frame.render_widget(self.block(" Search actions ", true), rows[0]);
                self.editor(frame, self.inset(rows[0], 1, 1), &query, false, true);
                let choices: Vec<_> = super::actions::search(&query.text)
                    .iter()
                    .map(|a| {
                        (
                            a.label.to_string(),
                            format!(
                                "{}{}",
                                a.hint,
                                a.shortcut
                                    .map(|c| format!(" · Ctrl+{}", c.to_ascii_uppercase()))
                                    .unwrap_or_default()
                            ),
                        )
                    })
                    .collect();
                if choices.is_empty() {
                    frame.render_widget(
                        Paragraph::new("No matching actions. Try a shorter name.")
                            .wrap(Wrap { trim: false })
                            .style(self.style(self.colors.muted))
                            .block(self.block(" Available actions ", false)),
                        rows[1],
                    );
                } else {
                    self.selectable_list(
                        frame,
                        app,
                        rows[1],
                        "Available actions",
                        choices,
                        selected,
                    );
                }
                for (_, hit) in &mut app.hits {
                    if let Hit::Row(index) = hit {
                        *hit = Hit::Choice(*index);
                    }
                }
                self.paragraph(
                    frame,
                    rows[2],
                    self.line("↑↓ Select · Enter Open · Esc Close", self.colors.muted),
                );
            }
            Dialog::Menu {
                title,
                description,
                items,
                selected,
            } => {
                frame.render_widget(self.block(format!(" {title} "), true), rect);
                let rows = Layout::vertical([
                    Constraint::Length({
                        use unicode_width::UnicodeWidthStr;
                        let width = rect.width.saturating_sub(4).max(1) as usize;
                        let lines = description
                            .lines()
                            .map(|line| line.width().div_ceil(width).max(1))
                            .sum::<usize>();
                        (lines as u16).clamp(1, rect.height.saturating_sub(7).clamp(1, 7))
                    }),
                    Constraint::Min(4),
                    Constraint::Length(1),
                ])
                .split(self.inset(rect, 2, 1));
                self.paragraph(
                    frame,
                    rows[0],
                    description
                        .lines()
                        .map(|s| self.line(s.to_owned(), self.colors.muted))
                        .collect::<Vec<_>>(),
                );
                let labels = items
                    .iter()
                    .map(|(label, hint, _)| (label.clone(), hint.clone()))
                    .collect();
                self.selectable_list(frame, app, rows[1], "Choose an option", labels, selected);
                // Menu rows select and activate using the same mouse convention as buttons.
                for (_, hit) in &mut app.hits {
                    if let Hit::Row(index) = hit {
                        *hit = Hit::Choice(*index);
                    }
                }
                // Appearance choices persist immediately; closing is not an undo.
                let close_label = if items.iter().any(|(_, _, action)| {
                    matches!(action, MenuAction::Appearance | MenuAction::ThemeMenu)
                }) {
                    "Close"
                } else {
                    "Cancel"
                };
                self.buttons(
                    frame,
                    app,
                    rows[2],
                    &[("Choose", "modal-submit"), (close_label, "modal-cancel")],
                );
            }
            Dialog::Connection(form) => {
                frame.render_widget(self.block(" Connect your model · Step 1 of 2 ", true), rect);
                let inner = self.inset(rect, 2, 1);
                let rows = Layout::vertical([
                    Constraint::Length(if inner.height >= 17 { 3 } else { 1 }),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(inner);
                self.paragraph(
                frame,
                rows[0],
                self.line(
                    if app
                        .job
                        .as_ref()
                        .is_some_and(|j| j.label == "Checking the connection")
                    {
                        "Checking the connection… Esc cancels."
                    } else if app.status_error {
                        &app.status
                    } else if inner.height >= 17 {
                        "Name this connection and enter the server address. Start the server in your model application first. Most local servers need no API key."
                    } else {
                        "Connect a running model server."
                    },
                    if app.status_error { self.colors.bad } else { self.colors.muted },
                ),
            );
                for (i, label) in [
                    "Connection name",
                    "Server address",
                    "API key variable (optional)",
                ]
                .iter()
                .enumerate()
                {
                    let field = rows[i + 1];
                    frame.render_widget(self.block(format!(" {label} "), form.focus == i), field);
                    self.editor(
                        frame,
                        self.inset(field, 1, 1),
                        &form.fields[i],
                        false,
                        form.focus == i,
                    );
                    app.hits.push((field, Hit::Field(i)));
                }
                self.paragraph(
                    frame,
                    rows[5],
                    self.line(
                        if form.focus == 2 {
                            "Enter the variable name, never the secret key."
                        } else {
                            "Tab fields · Ctrl+O model ID · Esc cancel"
                        },
                        self.colors.muted,
                    ),
                );
                self.buttons(
                    frame,
                    app,
                    rows[6],
                    &[
                        ("Check connection", "test-form"),
                        ("Enter model ID", "manual-model"),
                        ("Cancel", "modal-cancel"),
                    ],
                );
            }
            Dialog::PickModel {
                draft,
                models,
                selected,
            } => {
                frame.render_widget(self.block(" Choose your model · Step 2 of 2 ", true), rect);
                let rows = Layout::vertical([
                    Constraint::Length(2),
                    Constraint::Min(4),
                    Constraint::Length(1),
                ])
                .split(self.inset(rect, 2, 1));
                self.paragraph(
                    frame,
                    rows[0],
                    self.line(
                        format!("{} is connected. Choose the model to use.", draft.name),
                        self.colors.good,
                    ),
                );
                self.selectable_list(
                    frame,
                    app,
                    rows[1],
                    "Available models",
                    models
                        .iter()
                        .map(|m| (m.clone(), "Select to save this connection".into()))
                        .collect(),
                    selected,
                );
                self.buttons(
                    frame,
                    app,
                    rows[2],
                    &[
                        ("Use this model", "modal-submit"),
                        ("Cancel", "modal-cancel"),
                    ],
                );
            }
            Dialog::Input {
                title,
                hint,
                editor: input,
                multiline,
                ..
            } => {
                frame.render_widget(self.block(format!(" {title} "), true), rect);
                let rows = Layout::vertical([
                    Constraint::Length(4),
                    Constraint::Min(3),
                    Constraint::Length(1),
                ])
                .split(self.inset(rect, 2, 1));
                self.paragraph(frame, rows[0], self.line(hint, self.colors.muted));
                frame.render_widget(self.block(" Your text ", true), rows[1]);
                self.editor(frame, self.inset(rows[1], 1, 1), &input, multiline, true);
                self.buttons(
                    frame,
                    app,
                    rows[2],
                    &[
                        (
                            "Save",
                            if multiline {
                                "save-brief"
                            } else {
                                "modal-submit"
                            },
                        ),
                        ("Cancel", "modal-cancel"),
                    ],
                );
            }
            Dialog::Browser {
                kind,
                path,
                entries,
                selected,
            } => {
                let title = match kind {
                    BrowserKind::Project => "Choose a project folder",
                    BrowserKind::Model => "Import a GGUF model",
                    BrowserKind::Engine => "Choose engine",
                    BrowserKind::Runtime => "Choose runtime",
                };
                frame.render_widget(self.block(format!(" {title} "), true), rect);
                let rows = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Min(4),
                    Constraint::Length(1),
                ])
                .split(self.inset(rect, 2, 1));
                self.paragraph(
                    frame,
                    rows[0],
                    vec![
                    self.line(path.display().to_string(), self.colors.accent),
                    self.line(
                        if matches!(kind, BrowserKind::Project) {
                            "Enter opens a folder · Ctrl+S chooses the current folder"
                        } else {
                            "Enter opens folders or imports the selected file; files are not moved."
                        },
                        self.colors.muted,
                    ),
                ],
                );
                let labels = entries
                    .iter()
                    .map(|p| {
                        let parent = path.parent().is_some_and(|parent| p == parent);
                        (
                            if parent {
                                "..  Parent folder".into()
                            } else {
                                format!(
                                    "{} {}",
                                    if p.is_dir() { "▸" } else { "·" },
                                    p.file_name().unwrap_or_default().to_string_lossy()
                                )
                            },
                            if p.is_dir() {
                                "Folder".into()
                            } else {
                                p.metadata()
                                    .map(|m| human_bytes(m.len()))
                                    .unwrap_or_default()
                            },
                        )
                    })
                    .collect();
                self.selectable_list(frame, app, rows[1], "Files and folders", labels, selected);
                let choices = if matches!(kind, BrowserKind::Project) {
                    vec![
                        ("Use folder", "choose-folder"),
                        ("Open", "modal-submit"),
                        ("Type path", "type-path"),
                        ("Cancel", "modal-cancel"),
                    ]
                } else {
                    vec![
                        ("Open / Select", "modal-submit"),
                        ("Type path", "type-path"),
                        ("Cancel", "modal-cancel"),
                    ]
                };
                self.buttons(frame, app, rows[2], &choices);
            }
            Dialog::Confirm {
                title,
                body,
                selected,
                ..
            } => {
                frame.render_widget(
                    self.block(format!(" {title} · PgUp/PgDn details "), true),
                    rect,
                );
                let rows = Layout::vertical([Constraint::Min(5), Constraint::Length(2)])
                    .split(self.inset(rect, 2, 1));
                let stamp = format!("{title}:{}", body.len());
                if app.confirm_stamp != stamp {
                    app.modal_scroll = 0;
                    app.confirm_stamp = stamp;
                }
                frame.render_widget(
                    Paragraph::new(body)
                        .style(self.style(self.colors.text))
                        .wrap(Wrap { trim: false })
                        .scroll((app.modal_scroll, 0)),
                    rows[0],
                );
                let cols =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(rows[1]);
                for (i, label) in ["Cancel", "Continue"].iter().enumerate() {
                    frame.render_widget(
                        Paragraph::new(format!("  {label}  ")).style(
                            Style::default()
                                .bg(if selected == i {
                                    self.colors.accent
                                } else {
                                    self.colors.select
                                })
                                .fg(if selected == i {
                                    self.colors.accent_text
                                } else {
                                    self.colors.text
                                }),
                        ),
                        cols[i],
                    );
                    app.hits.push((cols[i], Hit::Choice(i)));
                }
            }
            Dialog::Notice {
                title,
                body,
                scroll,
            } => {
                frame.render_widget(self.block(format!(" {title} "), true), rect);
                let rows = Layout::vertical([Constraint::Min(5), Constraint::Length(1)])
                    .split(self.inset(rect, 2, 1));
                frame.render_widget(
                    Paragraph::new(display_text(&body))
                        .style(self.style(self.colors.text))
                        .wrap(Wrap { trim: false })
                        .scroll((scroll, 0)),
                    rows[0],
                );
                self.buttons(
                    frame,
                    app,
                    rows[1],
                    &[("Back · PgUp/PgDn to scroll", "modal-submit")],
                );
            }
            Dialog::ToolDetails { tool, scroll } => {
                frame.render_widget(
                    self.block(format!(" {} · {} ", tool.title, tool.status), true),
                    rect,
                );
                let rows = Layout::vertical([Constraint::Min(5), Constraint::Length(1)])
                    .split(self.inset(rect, 2, 1));
                let text = format!(
                    "REQUEST\n{}\n\nRESULT / FILE CHANGES\n{}",
                    tool_request(&tool.input),
                    tool.content
                );
                frame.render_widget(
                    Paragraph::new(display_text(&text))
                        .wrap(Wrap { trim: false })
                        .scroll((scroll, 0)),
                    rows[0],
                );
                self.buttons(frame, app, rows[1], &[("Close", "modal-submit")]);
            }
        }
    }

    fn permission_modal(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let Some((_, params)) = app.permissions.front() else {
            return;
        };
        let tool = &params["toolCall"];
        let rect = self.centered(area, 94, area.height.saturating_sub(2));
        frame.render_widget(Clear, rect);
        frame.render_widget(
            self.block(" Review this action ", true)
                .border_style(self.style(self.colors.warn)),
            rect,
        );
        let rows = Layout::vertical([
            Constraint::Length(5),
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(self.inset(rect, 2, 1));
        self.paragraph(
            frame,
            rows[0],
            vec![
            self.line(
                tool["title"]
                    .as_str()
                    .unwrap_or("Your model wants to use a tool"),
                self.colors.warn,
            ),
            self.line(
                format!(
                    "Project: {}",
                    app.session.as_ref().map(|s| s.cwd.as_str()).unwrap_or("")
                ),
                self.colors.muted,
            ),
            self.line(
                "Read the proposed command or change below. It runs with your account's access.",
                self.colors.text,
            ),
            self.line(
                format!(
                    "{} request(s) waiting · PgUp/PgDn scroll details",
                    app.permissions.len()
                ),
                self.colors.muted,
            ),
        ],
        );
        let details = format!(
            "{}\n{}",
            tool_request(&tool["rawInput"]),
            tool_content(&tool["content"])
        );
        frame.render_widget(
            Paragraph::new(display_text(&details))
                .wrap(Wrap { trim: false })
                .scroll((app.permission_scroll, 0))
                .block(self.block(" Proposed action / file changes ", false)),
            rows[1],
        );
        self.paragraph(
        frame,
        rows[2],
        self.line(
            "Allow this session accepts future tool requests in this conversation until you reconnect. Reject is selected initially. Ctrl+Y allows once; Ctrl+N rejects.",
            self.colors.muted,
        ),
    );
        let cols = Layout::horizontal([
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(40),
        ])
        .split(rows[3]);
        for (i, label) in ["Reject", "Allow once", "Allow this session"]
            .iter()
            .enumerate()
        {
            frame.render_widget(
                Paragraph::new(format!(" {label} ")).style(
                    Style::default()
                        .bg(if app.permission_selected == i {
                            self.colors.warn
                        } else {
                            self.colors.select
                        })
                        .fg(if app.permission_selected == i {
                            Palette::ink_on(self.colors.warn)
                        } else {
                            self.colors.text
                        }),
                ),
                cols[i],
            );
            app.hits.push((cols[i], Hit::Choice(i)));
        }
    }

    fn task(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let compact = area.height < 16;
        let rows = Layout::vertical([
            Constraint::Length(8),
            Constraint::Min(3),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(area);
        let mut summary = if compact {
            vec![]
        } else {
            vec![
                self.line("Inspect → Plan → Edit → Test → Results", self.colors.accent),
                self.line(
                    format!(
                        "Access: {} · {} configured check(s)",
                        app.preferences.access_policy.label(),
                        app.task_view.checks.len()
                    ),
                    self.colors.muted,
                ),
            ]
        };
        if let Some(t) = &app.task_view.task {
            let verification = app.task_view.verification.as_ref();
            let evidence = verification
                .map(|v| {
                    if v.requirements.is_empty() {
                        format!(
                            "No completion requirements set · latest check: {}",
                            t.check_status
                        )
                    } else {
                        format!(
                            "{} · {}/{} required checks current",
                            if v.behavioral_acceptance {
                                "Independent assertion passed"
                            } else if v.complete {
                                "Commands passed; behavioral coverage unknown"
                            } else {
                                "Work remains; press V for details"
                            },
                            v.requirements
                                .iter()
                                .filter(|r| r.status == "passed on current files")
                                .count(),
                            v.requirements.len(),
                        )
                    }
                })
                .unwrap_or_else(|| t.check_status.clone());
            summary.push(self.line(
                format!("Evidence: {evidence}"),
                if verification.is_some_and(|v| v.complete) {
                    self.colors.good
                } else {
                    self.colors.warn
                },
            ));
            summary.push(self.line(format!("Next: {}", t.next), self.colors.text));
            summary.push(self.line(
                format!("Now: {} · {} tracked edit(s)", t.phase, t.changes),
                self.colors.text,
            ));
            summary.push(self.line(
                format!("Goal: {}", crate::project::bounded(&t.goal, 180)),
                self.colors.text,
            ));
        } else {
            summary.push(self.line(
                "Your model's file changes and check results appear here.",
                self.colors.text,
            ));
            summary.push(self.line(
                "Choose Configure checks to tell Alt what should prove your project works.",
                self.colors.text,
            ));
            summary.push(self.line(
                "You can run checks, inspect evidence and undo edits without asking the model.",
                self.colors.muted,
            ));
        }
        frame.render_widget(
            Paragraph::new(summary)
                .wrap(Wrap { trim: false })
                .block(self.block(" What happened · What next ", false)),
            rows[0],
        );
        if app.task_view.changes.is_empty() {
            frame.render_widget(Paragraph::new("No tracked edits yet.\n\nEdits made with Alt's edit tool save their original contents here. Full access terminal commands can have effects outside the project and are not covered by file undo.\n\nMemory and evidence remain available after restarting Alt.").wrap(Wrap{trim:false}).block(self.block(" Changes and checkpoints ",false)),rows[1]);
        } else {
            self.selectable_list(
                frame,
                app,
                rows[1],
                "Changes · Enter reviews the diff",
                app.task_view
                    .changes
                    .iter()
                    .map(|c| {
                        (
                            format!(
                                "{} · {}{}",
                                c.path,
                                c.status,
                                if c.test_change { " · TEST CHANGE" } else { "" }
                            ),
                            c.reason.clone(),
                        )
                    })
                    .collect(),
                app.change_selected,
            );
        }
        self.buttons(
            frame,
            app,
            rows[2],
            &[
                ("Run checks again", "task-check"),
                ("Required checks", "verification-plan"),
                ("Verify all", "verification-run"),
                ("Configure checks", "task-configure"),
                ("Evidence", "task-evidence"),
                ("Refresh", "task-refresh"),
            ],
        );
        self.buttons(
            frame,
            app,
            rows[3],
            &[
                ("Diff", "task-diff"),
                ("Undo edit", "task-undo"),
                ("Undo task", "task-undo-all"),
                ("Memory", "task-memory"),
                ("Remember", "task-note"),
                ("Export", "task-export"),
            ],
        );
    }

    fn files(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);
        self.paragraph(
            frame,
            rows[0],
            vec![
                self.line(
                    format!(
                        "{} · filter: {}",
                        app.preferences.project.display(),
                        app.workbench.filter
                    ),
                    self.colors.text,
                ),
                self.line(
                    "Enter views · S finds text · G opens a line · E edits · N creates · D diffs",
                    self.colors.muted,
                ),
            ],
        );
        let items = app
            .filtered_files()
            .iter()
            .map(|p| ((*p).to_string(), "Project file".into()))
            .collect();
        self.selectable_list(
            frame,
            app,
            rows[1],
            "Project files",
            items,
            app.workbench.file_selected,
        );
        self.buttons(
            frame,
            app,
            rows[2],
            &[
                ("View", "file-view"),
                ("Edit", "file-edit"),
                ("New file", "file-new"),
                ("Filter", "files-search"),
                ("Find", "file-find"),
                ("Line", "file-line"),
                ("All diffs", "all-diffs"),
                ("Refresh", "files-refresh"),
            ],
        );
    }
    fn jobs(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let rows = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(6),
            Constraint::Min(4),
            Constraint::Length(2),
        ])
        .split(area);
        self.paragraph(
        frame,
        rows[0],
        self.line(
            if app.workbench.attached {
                "TERMINAL INPUT ACTIVE · Ctrl+] detaches · Ctrl+C interrupts the program"
            } else {
                "Terminal jobs · Enter attaches · N starts · S stops · R restarts · H health · L logs"
            },
            if app.workbench.attached {
                self.colors.accent
            } else {
                self.colors.muted
            },
        ),
    );
        let items = app
            .workbench
            .jobs
            .iter()
            .map(|r| {
                (
                    format!("{} · {}", r.spec.name, r.status),
                    format!(
                        "{} · {}",
                        if r.spec.keep {
                            "Keep running"
                        } else {
                            "While Alt is open"
                        },
                        r.id
                    ),
                )
            })
            .collect();
        self.selectable_list(
            frame,
            app,
            rows[1],
            "Managed jobs",
            items,
            app.workbench.job_selected,
        );
        let panel = self.block("Live terminal", app.workbench.attached);
        let inner = panel.inner(rows[2]);
        frame.render_widget(panel, rows[2]);
        frame.render_widget(
            Paragraph::new(display_text(&app.workbench.screen)).style(self.style(self.colors.text)),
            inner,
        );
        if app.workbench.terminal.is_some() && app.workbench.dimensions != (inner.width, inner.height)
        && app.terminal_input(serde_json::json!({"action":"resize","cols":inner.width.max(10),"rows":inner.height.max(2)})).is_ok(){app.workbench.dimensions=(inner.width,inner.height);}
        self.buttons(
            frame,
            app,
            rows[3],
            &[
                ("Start", "job-new"),
                ("Attach", "job-attach"),
                ("Detach", "job-detach"),
                ("Stop", "job-stop"),
                ("Restart", "job-restart"),
                ("Logs", "job-logs"),
                ("Health", "job-health"),
            ],
        );
    }
    fn context(&self, frame: &mut Frame, app: &mut App, area: Rect) {
        let rows = Layout::vertical([Constraint::Min(5), Constraint::Length(2)]).split(area);
        frame.render_widget(
            Paragraph::new(display_text(&app.workbench.context))
                .block(self.block("Context & pinned requirements", false))
                .wrap(Wrap { trim: false })
                .scroll((app.help_scroll, 0)),
            rows[0],
        );
        self.buttons(
            frame,
            app,
            rows[1],
            &[
                ("Pin requirement", "pin-requirement"),
                ("Unpin", "unpin-requirement"),
                ("Refresh", "context-refresh"),
            ],
        );
    }
}

pub fn startup(frame: &mut Frame, appearance: &Appearance) {
    Renderer::new(appearance).startup(frame);
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    Renderer::new(&app.preferences.appearance).draw(frame, app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Profile, Provider};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    fn setup() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::load(dir.path().to_path_buf(), "goose".into(), None).unwrap();
        app.preferences.project = dir.path().to_path_buf();
        (dir, app)
    }
    fn render(app: &mut App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[tokio::test]
    async fn compact_home_keeps_guidance_and_every_choice_reachable() {
        let (_dir, mut app) = setup();
        for (width, height) in [(60, 18), (80, 24)] {
            app.home_selected = 0;
            let text = render(&mut app, width, height);
            assert!(text.contains("Project:"));
            assert!(text.contains("Next:"));
            assert!(text.contains("Connect your first model"));
            assert!(text.contains("Ctrl+Q Quit"));
            for _ in 0..7 {
                app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
                    .unwrap();
            }
            let text = render(&mut app, width, height);
            assert!(text.contains("Keep a project brief"));
            assert!(text.contains("8 / 8"));
            let hint_row = text
                .lines()
                .position(|row| row.contains("Save goals,"))
                .unwrap() as u16;
            app.mouse(
                crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
                2,
                hint_row,
            )
            .unwrap();
            while app.job.is_some() {
                let event = app.job_rx.recv().await.unwrap();
                app.job_event(event).unwrap();
            }
            assert!(
                matches!(&app.dialog, Some(Dialog::Input { title, .. }) if title == "Project brief")
            );
            app.dialog = None;
        }
    }

    #[tokio::test]
    async fn navigation_focus_is_visible_and_sidebar_clicks_have_unique_targets() {
        let (_dir, mut app) = setup();
        for (width, height) in [(60, 18), (80, 24), (100, 24), (120, 40)] {
            app.set_page(Page::Home);
            app.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
                .unwrap();
            for _ in 0..10 {
                app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
                    .unwrap();
            }
            let text = render(&mut app, width, height);
            assert!(text.contains(if width < 85 {
                "› Memory"
            } else {
                "›·  Context & memory"
            }));
            app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
                .unwrap();
            assert_eq!(app.page, Page::Context);
            render(&mut app, width, height);
            let nav: Vec<_> = app
                .hits
                .iter()
                .filter_map(|(rect, hit)| {
                    if let Hit::Nav(page) = hit {
                        Some((*rect, *page))
                    } else {
                        None
                    }
                })
                .collect();
            for (rect, page) in &nav {
                let position = (rect.x, rect.y).into();
                let matches: Vec<_> = nav
                    .iter()
                    .filter(|(other, _)| other.contains(position))
                    .collect();
                assert_eq!(matches.len(), 1, "ambiguous mouse target for {page:?}");
            }
        }
    }

    #[tokio::test]
    async fn every_page_and_dialog_renders_at_supported_sizes() {
        let (_dir, mut app) = setup();
        for (width, height) in [(140, 45), (100, 32), (80, 24), (60, 18)] {
            for page in Page::ALL {
                app.set_page(page);
                let text = render(&mut app, width, height);
                assert!(text.contains("ALT"));
            }
            app.connection_wizard();
            render(&mut app, width, height);
            app.menu_action(MenuAction::Preset(1)).unwrap();
            render(&mut app, width, height);
            app.install_dialog(crate::runtime::Component::Inference);
            render(&mut app, width, height);
            app.edit_brief();
            render(&mut app, width, height);
            app.pending_refresh = None;
            while app.job.is_some() {
                let event = app.job_rx.recv().await.unwrap();
                app.job_event(event).unwrap();
            }
            app.open_browser(BrowserKind::Project, app.preferences.project.clone())
                .unwrap();
            while app.job.is_some() {
                let event = app.job_rx.recv().await.unwrap();
                app.job_event(event).unwrap();
            }
            render(&mut app, width, height);
            app.dialog = None;
            app.permissions.push_back((serde_json::json!(1),serde_json::json!({"toolCall":{"title":"Run checks","rawInput":{"command":"python3 check.py"}}})));
            let text = render(&mut app, width, height);
            assert!(text.contains("Reject"));
            assert!(text.contains("Allow once"));
            app.permissions.clear();
        }
    }
    #[tokio::test]
    async fn first_run_is_usable_without_config_and_errors_preserve_forms() {
        let (_dir, mut app) = setup();
        let text = render(&mut app, 120, 36);
        assert!(text.contains("Connect your first model"));
        assert!(text.contains("Choose a project folder"));
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        assert!(matches!(app.dialog, Some(Dialog::Menu { .. })));
        app.menu_action(MenuAction::Preset(1)).unwrap();
        if let Some(Dialog::Connection(form)) = &mut app.dialog {
            form.fields[1].replace("not-an-address");
        }
        let error = app
            .key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap_err();
        app.error(error);
        assert!(matches!(app.dialog, Some(Dialog::Notice { .. })));
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        assert!(
            matches!(&app.dialog,Some(Dialog::Connection(form)) if form.fields[1].text=="not-an-address")
        );
        app.pending_prompt = Some(super::super::drafts::Submission::new(
            "Please keep my original request".into(),
        ));
        app.composer.replace("Keep the next message too");
        app.workspace_update(crate::workspace::Update::Error("Server unavailable".into()))
            .unwrap();
        assert_eq!(app.composer.text, "Keep the next message too");
        assert_eq!(
            app.pending_prompt.as_ref().unwrap().text,
            "Please keep my original request"
        );
        app.dialog = None;
        app.workspace_update(crate::workspace::Update::Done(
            serde_json::json!({"stopReason":"end_turn"}),
        ))
        .unwrap();
        assert!(app.messages.back().unwrap().text.contains("No tools ran"));
        app.apply_update(&serde_json::json!({"update":{"sessionUpdate":"tool_call_update","toolCallId":"nonzero","status":"completed","rawOutput":{"exit_code":7}}}));
        assert_eq!(app.tools.last().unwrap().status, "failed");
        assert!(app.tools.last().unwrap().content.contains("Exit code: 7"));
    }
    #[test]
    fn corrupt_settings_are_preserved_and_setup_still_opens() {
        let dir = tempfile::tempdir().unwrap();
        let original = "[profiles\nnot valid TOML";
        std::fs::write(dir.path().join("config.toml"), original).unwrap();
        let mut app = App::load(dir.path().to_path_buf(), "goose".into(), None).unwrap();
        assert!(app.config.profiles.is_empty());
        assert!(render(&mut app, 120, 36).contains("preserved"));
        let backup = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .find(|p| {
                p.file_name()
                    .to_string_lossy()
                    .starts_with("config.toml.recovery-")
            })
            .unwrap();
        assert_eq!(std::fs::read_to_string(backup.path()).unwrap(), original);
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        assert!(render(&mut app, 120, 36).contains("Connect your first model"));
    }

    #[test]
    fn settings_persist_and_permission_typing_cannot_accidentally_allow() {
        let (dir, mut app) = setup();
        app.config
            .upsert(
                "test".into(),
                Profile {
                    provider: Provider::Openai,
                    endpoint: "http://127.0.0.1:8080/v1".into(),
                    model: "fixture".into(),
                    context_tokens: 8192,
                    max_turns: 12,
                    uncensored: false,
                    api_key_env: None,
                    local_model: None,
                    inference: None,
                },
            )
            .unwrap();
        app.setting_action(0).unwrap();
        assert_eq!(
            crate::config::Config::read(dir.path()).unwrap().profiles["test"].context_tokens,
            16384
        );
        app.permissions
            .push_back((serde_json::json!(1), serde_json::json!({})));
        app.key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE))
            .unwrap();
        assert_eq!(app.permissions.len(), 1);
        assert_eq!(app.permission_selected, 0);
        app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
            .unwrap();
        assert_eq!(app.permission_selected, 1);
    }
}
