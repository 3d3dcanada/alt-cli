// Color shims keep the historical constant names for a minimal diff; they read
// the per-frame theme instead of hard-coded values.
#![allow(non_snake_case)]

use super::{app::*, fx, input::Editor, motion, theme};
use crate::{display_text, models::human_bytes};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Widget,
        Wrap,
    },
};

// Colors read through the per-frame theme so every panel follows the user's
// choice without a rebuild. Stock names kept for a minimal diff.
fn BG() -> Color {
    theme::cur().bg
}
fn PANEL() -> Color {
    theme::cur().panel
}
fn TEXT() -> Color {
    theme::cur().text
}
fn MUTED() -> Color {
    theme::cur().muted
}
fn ACCENT() -> Color {
    theme::cur().accent
}
fn ACCENT2() -> Color {
    theme::cur().accent2
}
fn BORDER() -> Color {
    theme::cur().border
}
fn BORDER_FOCUSED() -> Color {
    theme::cur().border_focused
}
fn SELECT() -> Color {
    theme::cur().select
}
fn GOOD() -> Color {
    theme::cur().good
}
fn WARN() -> Color {
    theme::cur().warn
}
fn BAD() -> Color {
    theme::cur().bad
}

fn style(color: Color) -> Style {
    Style::default().fg(color)
}
fn block(title: impl Into<Line<'static>>, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(style(ACCENT()))
        .border_style(style(if focused {
            // The focus ring breathes toward the second accent while a task runs.
            motion::blend(BORDER_FOCUSED(), ACCENT2(), motion::breath() * 0.6)
        } else {
            BORDER()
        }))
        .style(Style::default().bg(PANEL()).fg(TEXT()))
}
fn line(text: impl Into<String>, color: Color) -> Line<'static> {
    Line::styled(display_text(&text.into()), style(color))
}
fn paragraph(frame: &mut Frame, area: Rect, text: impl Into<Text<'static>>) {
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .style(style(TEXT())),
        area,
    );
}
fn inset(area: Rect, x: u16, y: u16) -> Rect {
    area.inner(Margin {
        horizontal: x,
        vertical: y,
    })
}

fn button(
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
                .fg(if active { BG() } else { TEXT() })
                .bg(if active { ACCENT() } else { SELECT() }),
        ),
        area,
    );
    app.hits.push((area, Hit::Button(action)));
}
fn buttons(frame: &mut Frame, app: &mut App, area: Rect, items: &[(&str, &'static str)]) {
    let mut x = area.x;
    for (label, action) in items {
        let width = (label.chars().count() as u16 + 2).min(area.right().saturating_sub(x));
        if width < 3 {
            break;
        }
        let rect = Rect::new(x, area.y, width, 1);
        button(frame, app, rect, label, action, false);
        x = x.saturating_add(width + 1);
    }
}

/// A smooth horizontal bar: whole cells plus one eighth-block partial cell.
pub(crate) fn meter_bar(fill: f64, width: usize) -> String {
    const PARTS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
    let eighths = (fill.clamp(0.0, 1.0) * width as f64 * 8.0).round() as usize;
    let full = (eighths / 8).min(width);
    let mut bar = "█".repeat(full);
    if full < width {
        let part = eighths % 8;
        bar.push(if part == 0 { '·' } else { PARTS[part] });
        bar.push_str(&"·".repeat(width - full - 1));
    }
    bar
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    theme::set(*app.theme);
    app.frame = app.frame.wrapping_add(1);
    app.motion.tick(app.busy);
    app.hits.clear();
    frame.render_widget(
        Block::default().style(Style::default().bg(BG()).fg(TEXT())),
        area,
    );
    if area.width < 60 || area.height < 18 {
        paragraph(
            frame,
            inset(area, 2, 1),
            vec![
                line("ALT", ACCENT()),
                line("Make this terminal at least 60 columns × 18 rows.", TEXT()),
                line("Your work is saved. Ctrl+Q leaves Alt.", MUTED()),
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
        .split(inset(root[0], 2, 0));
    let th = theme::cur();
    paragraph(
        frame,
        header[0],
        vec![
            Line::from(vec![
                Span::styled("ALT", style(ACCENT()).add_modifier(Modifier::BOLD)),
                Span::styled(" // ", style(ACCENT2())),
                Span::styled("LOCAL WORKSPACE", style(MUTED())),
            ]),
            line(app.page.name(), TEXT()),
            line(crate::BUILD_LABEL, MUTED()),
        ],
    );
    // M04: the wordmark is a two-accent gradient.
    fx::gradient(
        frame.buffer_mut(),
        Rect::new(header[0].x, header[0].y, 7.min(header[0].width), 1),
        ACCENT(),
        ACCENT2(),
    );
    let model = app
        .session
        .as_ref()
        .filter(|_| app.page == Page::Chat)
        .map(|s| s.profile.model.clone())
        .or_else(|| app.current_profile().map(|(_, p)| p.model.clone()))
        .unwrap_or_else(|| "Choose a model to get started".into());
    // Connection LED, access badge and a context meter share one line; the
    // second line animates while a task runs so the state is visible at a glance.
    let meter = match (app.context_used, app.context_size) {
        (Some(used), Some(size)) if size > 0 => {
            // The bar glides to each new reading in eighth-cell steps.
            let fill = app.motion.glide("ctx", used.min(size) as f64 / size as f64);
            format!(
                " ctx ▕{}▏ {}K/{}K",
                meter_bar(fill, 10),
                used / 1024,
                size / 1024
            )
        }
        _ => String::new(),
    };
    let (state_text, state_color, spinner) = if app.busy {
        (
            if app.permissions.is_empty() {
                "Working · Esc stops"
            } else {
                "Waiting for your approval"
            },
            WARN(),
            th.spinner[motion::spin(th.spinner.len())],
        )
    } else if app.connecting {
        (
            "Connecting…",
            WARN(),
            th.spinner[motion::spin(th.spinner.len())],
        )
    } else if app.connected {
        ("Connected · saved locally", GOOD(), "")
    } else {
        ("Local-first · you choose the model", MUTED(), "")
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", if app.connected { th.led_on } else { th.led_off }),
                    style(if app.connected { GOOD() } else { MUTED() }),
                ),
                Span::styled(model, style(ACCENT())),
                Span::styled(
                    format!("  · {}", app.preferences.access_policy.label()),
                    style(ACCENT2()),
                ),
            ]),
            Line::from(vec![
                Span::styled(format!("{spinner} "), style(state_color)),
                Span::styled(state_text, style(state_color)),
                Span::styled(meter, style(MUTED())),
            ]),
        ])
        .alignment(ratatui::layout::Alignment::Right),
        header[1],
    );
    // Layout modes: Sidebar (vertical nav), TopTabs / BottomTabs (horizontal),
    // Split (sidebar plus a live activity column on wide terminals).
    let content = match app.layout {
        LayoutMode::Sidebar if area.width >= 85 => {
            let cols =
                Layout::horizontal([Constraint::Length(21), Constraint::Min(30)]).split(root[1]);
            nav(frame, app, cols[0]);
            inset(cols[1], 1, 0)
        }
        LayoutMode::Split if area.width >= 100 => {
            let cols = Layout::horizontal([
                Constraint::Length(21),
                Constraint::Min(40),
                Constraint::Length(38),
            ])
            .split(root[1]);
            nav(frame, app, cols[0]);
            activity_column(frame, app, inset(cols[2], 1, 0));
            inset(cols[1], 1, 0)
        }
        LayoutMode::BottomTabs if area.width >= 60 => {
            let parts =
                Layout::vertical([Constraint::Min(8), Constraint::Length(2)]).split(root[1]);
            tab_strip(frame, app, parts[1], area.width);
            inset(parts[0], 1, 0)
        }
        _ => {
            let parts =
                Layout::vertical([Constraint::Length(2), Constraint::Min(8)]).split(root[1]);
            tab_strip(frame, app, parts[0], area.width);
            inset(parts[1], 1, 0)
        }
    };
    match app.page {
        Page::Home => home(frame, app, content),
        Page::Chat => chat(frame, app, content),
        Page::Models => models(frame, app, content),
        Page::Connections => connections(frame, app, content),
        Page::Sessions => sessions(frame, app, content),
        Page::Settings => settings(frame, app, content),
        Page::Help => help(frame, content, app.help_scroll),
        Page::Task => task(frame, app, content),
        Page::Files => files(frame, app, content),
        Page::Jobs => jobs(frame, app, content),
        Page::Context => context(frame, app, content),
    }
    page_transition(frame, app, content);
    toasts(frame, app, content);
    footer(frame, app, root[2]);
    dialog_layer(frame, app, area);
    pointer_feedback(frame, app);
    splash(frame, app, area);
}

/// M04: the target under the mouse glows softly; a click flashes it.
fn pointer_feedback(frame: &mut Frame, app: &mut App) {
    let target = app.motion.hover.and_then(|(x, y)| {
        app.hits
            .iter()
            .rev()
            .find(|(rect, hit)| rect.contains((x, y).into()) && !matches!(hit, Hit::Composer))
            .map(|(rect, _)| *rect)
    });
    let buf = frame.buffer_mut();
    if let Some((rect, strength)) = app.motion.hover(target) {
        fx::tint(
            buf,
            rect,
            ACCENT(),
            0.16 * motion::stepped(strength, 3),
            PANEL(),
        );
    }
    if let Some((rect, t)) = app.motion.press() {
        fx::tint(
            buf,
            rect,
            ACCENT(),
            0.45 * (1.0 - motion::stepped(t, 3)),
            PANEL(),
        );
    }
}

/// M04: a short wordmark splash on launch (Full motion only, 720 ms). Any key
/// passes straight through and ends it; the interface then cascades in.
fn splash(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(t) = app.motion.splash() else {
        return;
    };
    let buf = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.reset();
            cell.bg = BG();
        }
    }
    const LOGO: [&str; 2] = ["▄▀█ █   ▀█▀", "█▀█ █▄▄  █ "];
    const TAGLINE: &str = "L O C A L   W O R K S P A C E";
    let reveal = motion::stepped(motion::ease_out_cubic(t / 0.45), 4);
    let top = area.y + area.height.saturating_sub(6) / 2;
    let logo_x = area.x + area.width.saturating_sub(11) / 2;
    for (row, text) in LOGO.iter().enumerate() {
        for (i, ch) in text.chars().enumerate() {
            let color = motion::blend(ACCENT(), ACCENT2(), i as f64 / 10.0);
            buf[(logo_x + i as u16, top + row as u16)]
                .set_char(ch)
                .set_fg(motion::blend(BG(), color, reveal))
                .set_bg(BG());
        }
    }
    let tag_x = area.x + area.width.saturating_sub(TAGLINE.len() as u16) / 2;
    buf.set_string(
        tag_x,
        top + 3,
        TAGLINE,
        Style::default()
            .fg(motion::blend(BG(), MUTED(), reveal))
            .bg(BG()),
    );
    let bar = meter_bar(motion::stepped(motion::ease_in_out_cubic(t), 12), 24);
    let bar_x = area.x + area.width.saturating_sub(24) / 2;
    buf.set_string(
        bar_x,
        top + 5,
        bar,
        Style::default()
            .fg(motion::blend(BG(), ACCENT(), reveal))
            .bg(BG()),
    );
}

/// M02: a new page cascades in: rows settle top to bottom, each sliding a few
/// columns from the right and fading up from the background (a staggered
/// entrance). Each row passes through two stages, so the whole transition
/// costs about three repaints of the page. Reduced keeps the fade; Off draws
/// in place.
fn page_transition(frame: &mut Frame, app: &mut App, content: Rect) {
    let index = Page::ALL.iter().position(|p| *p == app.page).unwrap_or(0) as u8;
    app.motion.page(index);
    let Some(t) = app.motion.progress(app.motion.page_since, motion::PAGE_MS) else {
        return;
    };
    let moves = app.motion.level.moves();
    let height = content.height.max(1) as f64;
    let buf = frame.buffer_mut();
    for y in content.top()..content.bottom() {
        // Rows start 0–45% into the transition and take 55% of it to settle.
        let delay = (y - content.y) as f64 / height * 0.45;
        let local = ((t - delay) / 0.55).clamp(0.0, 1.0);
        let (fade, dx) = match motion::stepped(motion::ease_out_cubic(local), 3) {
            s if s < 0.33 => (0.8, 4),
            s if s < 0.66 => (0.35, 1),
            _ => continue,
        };
        let row = Rect::new(content.x, y, content.width, 1);
        if moves {
            fx::slide_right(buf, row, dx.min(content.width / 10), BG());
        }
        fx::fade(buf, row, BG(), fade, TEXT());
    }
}

/// M03: notices float in at the top right of the page and fade away. They
/// echo the status line (which stays put) and never take clicks.
fn toasts(frame: &mut Frame, app: &mut App, content: Rect) {
    // A toast only ever shows the current status; any later status (an
    // error, progress, a direct update) retires it at once.
    if app
        .motion
        .toasts
        .last()
        .is_some_and(|t| t.text != app.status.trim() || app.status_error)
    {
        app.motion.toasts.clear();
    }
    if app.motion.toasts.is_empty() || content.width < 30 || content.height < 8 {
        return;
    }
    let width = content.width.saturating_sub(2).min(46);
    let now = app.motion.now();
    let moves = app.motion.level.moves();
    let mut y = content.y;
    for toast in app.motion.toasts.clone().iter().rev() {
        let glyph = match toast.tone {
            motion::Tone::Good => "✓ ",
            motion::Tone::Bad => "✕ ",
            motion::Tone::Info => "• ",
        };
        let mut text = format!("{glyph}{}", toast.text);
        if toast.count > 1 {
            text.push_str(&format!("  ×{}", toast.count));
        }
        let lines = textwrap_lines(&text, width.saturating_sub(4) as usize, 2);
        let height = lines.len() as u16 + 2;
        if y + height > content.bottom() {
            break;
        }
        let age = now - toast.born;
        // Four stages in and out keep a toast to a few small repaints.
        let enter = motion::stepped(motion::ease_out_cubic(age / motion::TOAST_EDGE_MS), 4);
        let leave = motion::stepped((motion::TOAST_MS - age) / motion::TOAST_EDGE_MS, 4);
        let shift = if moves {
            ((1.0 - enter) * (width as f64 * 0.5)).round() as u16
        } else {
            0
        };
        let x = content.right().saturating_sub(width + 1) + shift;
        let rect = Rect::new(x, y, width.min(content.right().saturating_sub(x)), height);
        let tone = match toast.tone {
            motion::Tone::Good => GOOD(),
            motion::Tone::Bad => BAD(),
            motion::Tone::Info => ACCENT(),
        };
        frame.render_widget(Clear, rect);
        frame.render_widget(
            Paragraph::new(
                lines
                    .into_iter()
                    .enumerate()
                    .map(|(i, l)| match l.strip_prefix(glyph).filter(|_| i == 0) {
                        Some(rest) => Line::from(vec![
                            Span::styled(glyph, style(tone).add_modifier(Modifier::BOLD)),
                            Span::styled(rest.to_owned(), style(TEXT())),
                        ]),
                        None => line(l, TEXT()),
                    })
                    .collect::<Vec<_>>(),
            )
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM | Borders::RIGHT)
                    .border_type(BorderType::Rounded)
                    .border_style(style(tone))
                    .style(Style::default().bg(PANEL()).fg(TEXT())),
            ),
            rect,
        );
        let buf = frame.buffer_mut();
        let fade = 1.0 - enter.min(leave);
        if fade > 0.0 {
            fx::fade(buf, rect, BG(), fade, TEXT());
        }
        fx::shadow(
            buf,
            rect,
            motion::blend(BG(), Color::Rgb(0, 0, 0), 0.55),
            0.5 * (1.0 - fade),
        );
        y += height + 1;
    }
}

/// Greedy word wrap into at most `max` lines; the last line ends in "…" when cut.
fn textwrap_lines(text: &str, width: usize, max: usize) -> Vec<String> {
    let width = width.max(8);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in display_text(text).split_whitespace() {
        let fits =
            current.chars().count() + word.chars().count() + usize::from(!current.is_empty())
                <= width;
        if !fits && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&word.chars().take(width).collect::<String>());
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.len() > max {
        lines.truncate(max);
        let last = &mut lines[max - 1];
        let keep: String = last.chars().take(width.saturating_sub(1)).collect();
        *last = format!("{keep}…");
    }
    lines
}

/// Identity of the dialog on screen; a new identity replays the entrance.
fn dialog_key(app: &App) -> Option<String> {
    if let Some((id, _)) = app.permissions.front() {
        return Some(format!("permission:{id}"));
    }
    app.dialog.as_ref().map(|dialog| match dialog {
        Dialog::Menu { title, .. } => format!("menu:{title}"),
        other => format!("{:?}", std::mem::discriminant(other)),
    })
}

/// M02: dialogs dim the page behind them, cast a shadow and pop in (scale
/// 86% → 100% with a slight overshoot); closing fades them back out.
fn dialog_layer(frame: &mut Frame, app: &mut App, area: Rect) {
    let key = dialog_key(app);
    let entrance = app.motion.modal(key.clone());
    if key.is_none() {
        if let Some((t, retained)) = app.motion.closing() {
            fx::fade_out(
                frame.buffer_mut(),
                retained.rect,
                &retained.cells,
                motion::stepped(motion::ease_out_cubic(t), 2),
                TEXT(),
                BG(),
            );
        }
        return;
    }
    app.hits.clear();
    let shade = motion::blend(BG(), Color::Rgb(0, 0, 0), 0.55);
    // The backdrop dims in one step; only the dialog itself animates.
    fx::fade(frame.buffer_mut(), area, shade, 0.5, TEXT());
    let entrance = entrance.map(|t| motion::stepped(t, 3));
    // Only the entrance needs what lay beneath; a settled dialog costs no
    // full-screen snapshot.
    let moves = app.motion.level.moves();
    let before = (entrance.is_some() && moves).then(|| fx::snapshot(frame.buffer_mut(), area));
    app.motion.dialog_rect = None;
    modal(frame, app, area);
    let buf = frame.buffer_mut();
    let Some(rect) = app.motion.dialog_rect.map(|r| r.intersection(area)) else {
        return;
    };
    let visible = match (entrance, &before) {
        (Some(t), Some(before)) => {
            let visible = fx::scaled(rect, 0.86 + 0.14 * motion::ease_out_back(t));
            fx::clip_to(buf, area, before, rect, visible);
            // A ring on the growing edge keeps the reveal a clean box.
            if visible != rect {
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(style(BORDER_FOCUSED()))
                    .render(visible, buf);
            }
            visible
        }
        _ => rect,
    };
    if let Some(t) = entrance {
        fx::fade(
            buf,
            visible,
            BG(),
            (1.0 - motion::ease_out_cubic(t)) * 0.7,
            TEXT(),
        );
    } else {
        app.motion.modal_retained = Some(motion::Retained {
            rect,
            cells: fx::snapshot(buf, rect),
        });
    }
    fx::shadow(buf, visible, shade, 0.7);
}

/// Live activity/context column; rendered beside Chat and in Split layout.
fn activity_column(frame: &mut Frame, app: &mut App, area: Rect) {
    let th = theme::cur();
    let activity = area;
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
    paragraph(
        frame,
        inset(chunks[0], 1, 0),
        vec![
            Line::from(vec![
                Span::styled("ACTIVITY", style(ACCENT())),
                Span::styled(" & ", style(MUTED())),
                Span::styled("CONTEXT", style(ACCENT2())),
            ]),
            line(context, MUTED()),
            line(
                format!("Project brief: {} chars", app.brief.chars().count()),
                MUTED(),
            ),
            line(
                match (app.trust_session, app.trust_host) {
                    (_, true) => "Project and terminal actions trusted this session",
                    (true, false) => "Project actions trusted; terminal still asks",
                    _ => "Tool actions request approval",
                },
                if app.trust_session || app.trust_host {
                    WARN()
                } else {
                    MUTED()
                },
            ),
        ],
    );
    let flashes: Vec<Option<f64>> = app
        .tools
        .iter()
        .map(|tool| app.motion.flash(&tool.id, &tool.status))
        .collect();
    let spinner = th.spinner[motion::spin(th.spinner.len())];
    let items = app
        .tools
        .iter()
        .map(|tool| {
            let (led, status_color) = match tool.status.as_str() {
                "completed" => (th.led_on.to_string(), GOOD()),
                "failed" => (th.led_on.to_string(), BAD()),
                _ if app.busy && app.motion.level.fades() => (spinner.to_string(), WARN()),
                _ => (th.led_off.to_string(), WARN()),
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{led} "), style(status_color)),
                    Span::styled(tool.title.clone(), style(TEXT())),
                ]),
                line(format!("  {}", tool.status), status_color),
            ])
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No tool actions yet.\n\nCommands, file changes, and results will appear here.",
            )
            .wrap(Wrap { trim: false })
            .block(block(" Tools ", app.tools_focus)),
            chunks[1],
        );
    } else {
        let mut state = ListState::default().with_selected(Some(app.tool_selected));
        frame.render_stateful_widget(
            List::new(items)
                .highlight_style(Style::default().bg(SELECT()))
                .block(block(" Tools · Enter for evidence ", app.tools_focus)),
            chunks[1],
            &mut state,
        );
        let inner = inset(chunks[1], 1, 1);
        let offset = state.offset();
        for i in offset..app.tools.len() {
            let y = inner.y + (i - offset) as u16 * 2;
            if y >= inner.bottom() {
                break;
            }
            let row = Rect::new(inner.x, y, inner.width, 2.min(inner.bottom() - y));
            if let Some(t) = flashes.get(i).copied().flatten() {
                let color = match app.tools[i].status.as_str() {
                    "completed" => GOOD(),
                    "failed" => BAD(),
                    _ => WARN(),
                };
                let fade = 1.0 - motion::stepped(motion::ease_out_cubic(t), 4);
                fx::tint(frame.buffer_mut(), row, color, 0.42 * fade, PANEL());
            }
            app.hits.push((row, Hit::Row(i)));
        }
    }
    buttons(
        frame,
        app,
        inset(chunks[2], 1, 1),
        &[("Evidence", "tool-details"), ("Task", "task")],
    );
}

/// Horizontal page tab strip shared by the TopTabs and BottomTabs layouts.
fn tab_strip(frame: &mut Frame, app: &mut App, area: Rect, width: u16) {
    let labels = [
        "Home", "Chat", "Models", "Links", "Saved", "Settings", "Help", "Task", "Files", "Jobs",
        "Memory",
    ];
    let visible = (width as usize / 10).clamp(3, 7);
    let active = Page::ALL.iter().position(|p| *p == app.page).unwrap_or(0);
    let start = active
        .saturating_sub(visible / 2)
        .min(Page::ALL.len() - visible);
    let cols = Layout::horizontal(vec![Constraint::Fill(1); visible]).split(area);
    for (column, i) in (start..start + visible).enumerate() {
        let page = &Page::ALL[i];
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(
                        "{}{} ",
                        if *page == app.page {
                            theme::cur().pointer
                        } else {
                            ' '
                        },
                        if i < 9 {
                            (i + 1).to_string()
                        } else if i == 9 {
                            "0".to_string()
                        } else {
                            "·".to_string()
                        }
                    ),
                    style(if *page == app.page {
                        ACCENT2()
                    } else {
                        MUTED()
                    }),
                ),
                Span::styled(
                    labels[i].to_string(),
                    style(if *page == app.page { ACCENT() } else { MUTED() }),
                ),
            ])),
            cols[column],
        );
        app.hits.push((cols[column], Hit::Nav(*page)));
    }
}

fn nav(frame: &mut Frame, app: &mut App, area: Rect) {
    let inner = inset(area, 1, 0);
    frame.render_widget(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(style(BORDER())),
        area,
    );
    let spacing = if inner.height >= 30 { 2 } else { 1 };
    let th = theme::cur();
    for (i, page) in Page::ALL.iter().enumerate() {
        let rect = Rect::new(
            inner.x,
            inner.y + i as u16 * spacing,
            inner.width.saturating_sub(1),
            2,
        );
        let active = *page == app.page;
        let focused = app.nav_focus && app.nav_index == i;
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", if active { th.pointer } else { ' ' }),
                    style(if active || focused {
                        ACCENT()
                    } else {
                        BORDER()
                    }),
                ),
                Span::styled(
                    if i < 9 {
                        (i + 1).to_string()
                    } else if i == 9 {
                        "0".into()
                    } else {
                        "·".into()
                    },
                    style(if active || focused {
                        ACCENT2()
                    } else {
                        MUTED()
                    }),
                ),
                Span::styled(
                    format!(" {}", page.name()),
                    if active || focused {
                        Style::default()
                            .bg(SELECT())
                            .fg(ACCENT())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().bg(BG()).fg(MUTED())
                    },
                ),
            ]))
            .style(Style::default().bg(if active || focused {
                SELECT()
            } else {
                BG()
            })),
            rect,
        );
        app.hits.push((rect, Hit::Nav(*page)));
    }
    if inner.height > 22 {
        let start = inner.bottom().saturating_sub(5);
        paragraph(
            frame,
            Rect::new(inner.x, start, inner.width, 5),
            vec![
                line("ON THIS COMPUTER", MUTED()),
                line(format!("{} RAM", human_bytes(app.hardware.ram)), TEXT()),
                line(format!("{} CPU threads", app.hardware.threads), TEXT()),
                line("Ctrl+P  Actions", MUTED()),
                line("F1      Help", MUTED()),
            ],
        );
    }
}

fn footer(frame: &mut Frame, app: &mut App, area: Rect) {
    let inner = inset(area, 2, 0);
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
        // Progress glides between reports; a fresh job starts from empty.
        let ratio = if ratio <= 0.0 {
            app.motion.glide("job", 0.0);
            0.0
        } else {
            app.motion.glide("job", ratio)
        };
        let progress = Rect::new(inner.x, inner.y, inner.width.saturating_sub(10), 1);
        frame.render_widget(
            Gauge::default()
                .ratio(ratio.clamp(0.0, 1.0))
                .label(label)
                .gauge_style(Style::default().fg(ACCENT()).bg(SELECT())),
            progress,
        );
        if job.progress.total == 0 && app.motion.level.moves() {
            shimmer(frame.buffer_mut(), progress, app.motion.now());
        }
        let stage = job.progress.stage.clone();
        button(
            frame,
            app,
            Rect::new(progress.right() + 1, inner.y, 9, 1),
            "Pause",
            "pause-job",
            false,
        );
        paragraph(
            frame,
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
            line(stage, MUTED()),
        );
        inner.y + 3
    } else {
        inner.y
    };
    paragraph(
        frame,
        Rect::new(inner.x, status_y, inner.width, 1),
        line(
            app.status.clone(),
            if app.status_error { BAD() } else { TEXT() },
        ),
    );
    let hint = if app.nav_focus {
        "↑↓ Choose page   Enter Open   Tab Return"
    } else {
        match app.page {
            Page::Files => "↑↓ Select  Enter View  E Edit  S Find  G Line  / Filter",
            Page::Jobs => "Enter Attach  Ctrl+] Detach  N Start  S Stop  R Restart  L Logs",
            Page::Context => "P Pin requirement  U Unpin  R Refresh  PgUp/PgDn Scroll",
            Page::Home => {
                "↑↓ Choose   Enter Open   Tab Navigation   Ctrl+P Quick actions   Ctrl+Q Quit"
            }
            Page::Chat => {
                "Enter Send   Alt+Enter / Ctrl+J New line   Esc Stop   Ctrl+T Activity   Ctrl+B Brief"
            }
            Page::Models => {
                "←→ Library / Server / Hub   ↑↓ Select   Enter Use/Open   R Refresh   I Import   S Search"
            }
            Page::Connections => "↑↓ Select   Enter Use   A Add   E Edit   T Test   Delete Remove",
            Page::Sessions => {
                "↑↓ Select   Enter Resume   / Search   R Rename   E Export   Delete Archive"
            }
            Page::Task => {
                "↑↓ Change   Enter Diff   R Run checks   U Undo   C Configure   M Memory   E Evidence"
            }
            Page::Settings => {
                "←→ Category   ↑↓ Choose setting   Enter Change   Settings are saved automatically"
            }
            Page::Help => {
                "Alt+1…8 Switch page   Tab Navigation   Ctrl+P Quick actions   Ctrl+Q Quit"
            }
        }
    };
    paragraph(
        frame,
        Rect::new(inner.x, status_y + 1, inner.width, 1),
        line(hint, MUTED()),
    );
}

/// A soft band of light sweeping across a bar whose length is unknown.
fn shimmer(buf: &mut ratatui::buffer::Buffer, area: Rect, now: f64) {
    let span = area.width as f64 + 16.0;
    let centre = (now / 1500.0).fract() * span - 8.0;
    for x in area.left()..area.right() {
        let distance = ((x - area.x) as f64 - centre).abs();
        if distance < 8.0 {
            let strength = 0.55 * (1.0 - distance / 8.0).powi(2);
            fx::tint(
                buf,
                Rect::new(x, area.y, 1, 1),
                ACCENT(),
                strength,
                SELECT(),
            );
        }
    }
}

/// Status cards plus a two-column grid of action cards (H01, harvested from the
/// final-pass branch and drawn through the active theme). Indexes match the
/// Start-here list, so keyboard, mouse and actions are unchanged.
fn dashboard(frame: &mut Frame, app: &mut App, area: Rect, labels: &[(&str, &str)]) {
    let rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(4 * labels.len().div_ceil(2) as u16),
        Constraint::Min(0),
    ])
    .split(area);
    let cards = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(rows[0]);
    let (model, model_detail) = app
        .current_profile()
        .map(|(_, p)| {
            (
                p.model.clone(),
                format!(
                    "{}K context · thinking {}",
                    p.context_tokens / 1024,
                    match p.effective_inference().effective_thinking() {
                        crate::inference::Thinking::Default => "default".to_string(),
                        other => other.label().to_lowercase(),
                    }
                ),
            )
        })
        .unwrap_or_else(|| {
            (
                "Choose your first model".into(),
                "Connect → local or hosted".into(),
            )
        });
    let project = app
        .preferences
        .project
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| app.preferences.project.display().to_string());
    let project_detail = if app.project_missing {
        "Folder missing; choose another".to_string()
    } else {
        format!("{} check(s) configured", app.task_view.checks.len())
    };
    let tools = if app.engine_ready() {
        "Agent engine ready"
    } else {
        "Agent engine missing"
    };
    let theme = theme::cur();
    let statuses = [
        (
            " ◈ MODEL ",
            model,
            model_detail,
            if app.config.profiles.is_empty() {
                WARN()
            } else {
                GOOD()
            },
        ),
        (
            " ▣ PROJECT ",
            project,
            project_detail,
            if app.project_missing { BAD() } else { TEXT() },
        ),
        (
            " ◇ TOOLS ",
            tools.to_string(),
            app.preferences.access_policy.label().to_string(),
            if app.engine_ready() { GOOD() } else { WARN() },
        ),
    ];
    for (i, (title, value, detail, color)) in statuses.into_iter().enumerate() {
        frame.render_widget(
            Paragraph::new(vec![line(value, color), line(detail, MUTED())])
                .block(block(title, false)),
            cards[i],
        );
    }
    let grid = rows[1];
    let columns = Layout::horizontal([Constraint::Ratio(1, 2); 2]).split(grid);
    let icons = ['◈', '◇', '▣', '✓', '⇄', '✦', '⚙', '≡'];
    for (i, (title, hint)) in labels.iter().enumerate() {
        let column = columns[i % 2];
        let y = grid.y + (i / 2) as u16 * 4;
        if y + 4 > grid.bottom() {
            break;
        }
        let rect = Rect::new(column.x, y, column.width, 4);
        let selected = i == app.home_selected && !app.nav_focus;
        let card = block(
            format!(
                " {} {} ",
                icons.get(i).copied().unwrap_or(theme.pointer),
                title
            ),
            selected,
        );
        let card = if selected {
            card.style(Style::default().bg(SELECT()).fg(TEXT()))
        } else {
            card
        };
        frame.render_widget(
            Paragraph::new(line(*hint, if selected { TEXT() } else { MUTED() }))
                .wrap(Wrap { trim: true })
                .block(card),
            rect,
        );
        app.hits.push((rect, Hit::Home(i)));
    }
    if rows[2].height >= 6 {
        let cols = Layout::horizontal([Constraint::Percentage(52), Constraint::Percentage(48)])
            .split(rows[2]);
        frame.render_widget(
            Paragraph::new(vec![
                line("1  Describe the result you want.", TEXT()),
                line("2  Review commands and file changes.", TEXT()),
                line("3  Check the results together.", TEXT()),
                line(
                    "Esc stops a task · conversations are saved as you work",
                    MUTED(),
                ),
            ])
            .wrap(Wrap { trim: false })
            .block(block(" How it works ", false)),
            cols[0],
        );
        let context = app
            .current_profile()
            .map(|(_, p)| p.context_tokens)
            .unwrap_or(app.preferences.context_tokens);
        frame.render_widget(
            Paragraph::new(vec![
                line(
                    format!("Context budget: {}K tokens", context / 1024),
                    ACCENT(),
                ),
                line(
                    "Ctrl+P → Probe this model suggests thinking and output settings.",
                    TEXT(),
                ),
                line(
                    "Hosted APIs: Connect → a hosted API or another server preset.",
                    MUTED(),
                ),
            ])
            .wrap(Wrap { trim: false })
            .block(block(" Models and settings ", false)),
            cols[1],
        );
    }
}

fn home(frame: &mut Frame, app: &mut App, area: Rect) {
    let parts = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(8),
        Constraint::Length(2),
    ])
    .split(area);
    let connection = app
        .current_profile()
        .map(|(name, p)| format!("Model: {} · {}", name, p.model))
        .unwrap_or_else(|| "1  Choose a model connection to get started".into());
    paragraph(
        frame,
        parts[0],
        vec![
            Line::styled(
                "Make something. Understand it. Make it work.",
                style(TEXT()).add_modifier(Modifier::BOLD),
            ),
            line(
                "Describe your goal in ordinary language. Alt helps with the steps.",
                MUTED(),
            ),
            line(connection, ACCENT()),
            line(
                if app.project_missing {
                    format!(
                        "Project folder missing: {} · pick a folder from Start here",
                        app.preferences.project.display()
                    )
                } else {
                    format!("Project: {}", app.preferences.project.display())
                },
                if app.project_missing { BAD() } else { TEXT() },
            ),
            line(
                if app.engine_ready() {
                    "Agent engine ready · conversations stay on this computer"
                } else {
                    "Agent engine missing · install it below when you are ready"
                },
                if app.engine_ready() { GOOD() } else { WARN() },
            ),
        ],
    );
    let labels = [
        (
            if app.config.profiles.is_empty() {
                "Connect your first model"
            } else {
                "Start a new conversation"
            },
            "Tell the assistant what you want to accomplish.",
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
            "Choose how to test changes before asking your model to work.",
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
    // H01: graphical dashboard on roomy terminals; the list stays at 80×24.
    if parts[1].width >= 96 && parts[1].height >= 26 {
        dashboard(frame, app, parts[1], &labels);
        paragraph(
            frame,
            parts[2],
            line(
                "Tip: your assistant can be wrong. Tool results show what actually happened.",
                MUTED(),
            ),
        );
        return;
    }
    let (choices, guidance) = if parts[1].height >= 22 {
        let areas = Layout::vertical([Constraint::Length(18), Constraint::Min(4)]).split(parts[1]);
        (areas[0], Some(areas[1]))
    } else {
        (parts[1], None)
    };
    let rows = labels
        .iter()
        .map(|(title, hint)| ListItem::new(vec![line(*title, TEXT()), line(*hint, MUTED())]))
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(app.home_selected));
    frame.render_stateful_widget(
        List::new(rows)
            .highlight_style(Style::default().bg(SELECT()))
            .highlight_symbol("› ")
            .block(block(" Start here ", !app.nav_focus)),
        choices,
        &mut state,
    );
    let list = inset(choices, 1, 1);
    let offset = state.offset();
    for i in offset..labels.len() {
        let y = list.y + (i - offset) as u16 * 2;
        if y >= list.bottom() {
            break;
        }
        app.hits.push((
            Rect::new(list.x, y, list.width, 2.min(list.bottom() - y)),
            Hit::Home(i),
        ));
    }
    if let Some(guidance) = guidance {
        let cols = Layout::horizontal([Constraint::Percentage(52), Constraint::Percentage(48)])
            .split(guidance);
        frame.render_widget(
            Paragraph::new(vec![
                line("1  Describe the result you want.", TEXT()),
                line("2  Review commands and file changes.", TEXT()),
                line("3  Check the results together.", TEXT()),
                line("", TEXT()),
                line(
                    "You can stop a task with Esc. Conversations are saved as you work.",
                    MUTED(),
                ),
            ])
            .wrap(Wrap { trim: false })
            .block(block(" How it works ", false)),
            cols[0],
        );
        let context = app
            .current_profile()
            .map(|(_, p)| p.context_tokens)
            .unwrap_or(app.preferences.context_tokens);
        frame.render_widget(
            Paragraph::new(vec![
                line(
                    format!("Context budget: {}K tokens", context / 1024),
                    ACCENT(),
                ),
                line(
                    "Start with a small 3B–4B quantized model. More context needs more memory.",
                    TEXT(),
                ),
                line("", TEXT()),
                line(
                    "An existing model server can use your GPU. Alt's built-in runtime uses CPU.",
                    MUTED(),
                ),
            ])
            .wrap(Wrap { trim: false })
            .block(block(" For smaller computers ", false)),
            cols[1],
        );
    }
    paragraph(
        frame,
        parts[2],
        line(
            "Tip: your assistant can be wrong. Tool results show what actually happened.",
            MUTED(),
        ),
    );
}

fn chat(frame: &mut Frame, app: &mut App, area: Rect) {
    let th = theme::cur();
    let compact = area.height < 12;
    let rows = Layout::vertical([
        Constraint::Min(if compact { 3 } else { 5 }),
        Constraint::Length(if compact { 3 } else { 5 }),
        Constraint::Length(1),
    ])
    .split(area);
    let (conversation, activity) = if rows[0].width >= 84 {
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
                line("What would you like to make or figure out?", TEXT()),
                line("", TEXT()),
                line("Examples", ACCENT()),
                line("• Explain how this project works.", MUTED()),
                line("• Add a settings screen with a dark theme.", MUTED()),
                line("• Help me understand and fix this error.", MUTED()),
                line("• Run the checks and explain any failures.", MUTED()),
                line("", TEXT()),
                line("Your first message connects the selected model.", MUTED()),
                line("No model selected? Open Connections first.", MUTED()),
            ]);
        }
        let newest = app.messages.len().saturating_sub(1);
        let reveal = app
            .messages
            .back()
            .filter(|m| m.role != "You" && m.role != "Notice")
            .map(|m| {
                let len = m.text.chars().count();
                (len, app.motion.reveal(newest, len, app.busy))
            });
        for (index, message) in app.messages.iter().enumerate() {
            let partial = reveal
                .filter(|(len, shown)| index == newest && shown < len)
                .map(|(_, shown)| message.text.chars().take(shown).collect::<String>());
            let body = partial.as_deref().unwrap_or(&message.text);
            lines.push(line("", TEXT()));
            let (badge_color, led) = if message.role == "You" {
                (ACCENT2(), th.led_on)
            } else if message.role == "Notice" {
                (WARN(), th.led_on)
            } else {
                (ACCENT(), th.led_on)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{led} "), style(badge_color)),
                Span::styled(
                    format!(" {} ", message.role.to_uppercase()),
                    style(badge_color).add_modifier(Modifier::BOLD),
                ),
            ]));
            let mut code = false;
            for text in body.lines() {
                if text.starts_with("```") {
                    code = !code;
                    lines.push(line(text, MUTED()));
                    continue;
                }
                lines.push(Line::styled(
                    text.to_string(),
                    if code {
                        Style::default().bg(SELECT()).fg(TEXT())
                    } else if text.starts_with('#') {
                        style(ACCENT())
                    } else {
                        style(TEXT())
                    },
                ));
            }
            if partial.is_some() {
                // A soft cursor rides the end of a reply that is still arriving.
                match lines.last_mut() {
                    Some(last) if !body.ends_with('\n') => {
                        last.spans.push(Span::styled("▍", style(ACCENT())))
                    }
                    _ => lines.push(line("▍", ACCENT())),
                }
            }
        }
        if app.busy {
            let frame_index = motion::spin(th.spinner.len());
            lines.push(Line::from(vec![
                Span::styled(format!("{} ", th.spinner[frame_index]), style(WARN())),
                Span::styled(
                    format!(
                        "Working · {}s · {}",
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
                    style(WARN()),
                ),
            ]));
        }
        if let Some(s) = &app.inference_status {
            lines.push(line(
                format!(
                    "Calls left: {} · generated tokens left: {}",
                    s.remaining_requests
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "no limit".into()),
                    s.remaining_generated_tokens
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "no limit".into())
                ),
                MUTED(),
            ));
            lines.push(line(
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
                MUTED(),
            ));
        }
        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block(" Conversation ", !app.tools_focus && !app.nav_focus));
        let count = paragraph
            .line_count(conversation.width)
            .min(u16::MAX as usize) as u16;
        let scroll = count
            .saturating_sub(conversation.height.saturating_sub(2))
            .saturating_sub(app.chat_scroll);
        frame.render_widget(paragraph.scroll((scroll, 0)), conversation);
    }
    if let Some(activity) = activity {
        activity_column(frame, app, activity);
    }
    let composer_block = block(
        if app.busy {
            " Next message · send after this task finishes "
        } else {
            " Describe what you want · Enter sends "
        },
        !app.tools_focus && !app.nav_focus,
    );
    frame.render_widget(composer_block, rows[1]);
    let input = inset(rows[1], 1, 1);
    editor(
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
    buttons(frame, app, rows[2], &choices);
}

fn selectable_list(
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
            let mut lines = vec![line(title, TEXT())];
            if row_height > 1 {
                lines.push(line(hint, MUTED()));
            }
            ListItem::new(lines)
        })
        .collect::<Vec<_>>();
    frame.render_stateful_widget(
        List::new(items)
            .block(block(format!(" {title} "), !app.nav_focus))
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(SELECT())),
        area,
        &mut state,
    );
    let inner = inset(area, 1, 1);
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

fn models(frame: &mut Frame, app: &mut App, area: Rect) {
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
                        ACCENT()
                    } else {
                        MUTED()
                    })
                    .bg(if app.model_tab == i { SELECT() } else { BG() }),
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
    paragraph(frame, rows[1], line(intro, MUTED()));
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
            .map(|m| (m.id.clone(), m.describe()))
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
                .block(block(" Model library ", true)),
            rows[2],
        );
    } else {
        selectable_list(
            frame,
            app,
            rows[2],
            "Choose a model",
            items,
            app.model_selected,
        );
    }
    let details = match app.model_tab {
        0 => app
            .artifacts
            .get(app.model_selected)
            .map(|m| {
                format!(
                    "{}\nLicense: {}\n{} · CPU runtime {}",
                    m.path.display(),
                    m.license,
                    if m.uncensored_claim {
                        "Publisher labels this variant uncensored/abliterated"
                    } else {
                        "Uncensored status not established"
                    },
                    if app.runtime_ready() {
                        "installed"
                    } else {
                        "not installed"
                    }
                )
            })
            .unwrap_or_default(),
        1 => app
            .server_models
            .get(app.model_selected)
            .map(|m| {
                format!(
                    "{}\n{}\nConnection: {} · Selecting applies to new conversations.",
                    m.id,
                    m.blurb(),
                    app.server_source.as_deref().unwrap_or("none")
                )
            })
            .unwrap_or_else(|| "Open Connections to add a server, then Refresh.".into()),
        _ => {
            let filter = if app.variants_only {
                "Variant filter ON (publisher claims)"
            } else {
                "Variant filter OFF"
            };
            app.hub_files.get(app.model_selected).map(|f|format!("{}\n{} · download {}\nRunning a model needs additional memory for context and working buffers.",f.repo,filter,human_bytes(f.bytes))).unwrap_or_else(||format!("Search: {}\n{filter}\nOnly complete GGUF files with published SHA256 metadata are offered.",app.hub_query))
        }
    };
    paragraph(frame, rows[3], line(details, MUTED()));
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
    buttons(frame, app, rows[4], &buttons_list);
}

fn connections(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .split(area);
    paragraph(
        frame,
        rows[0],
        vec![
            line("Where your model runs", TEXT()),
            line(
                "Choose a saved connection, or add one with the guided setup.",
                MUTED(),
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
        frame.render_widget(Paragraph::new("No connections yet.\n\nChoose Add connection below. Alt will help you select your application, check its address, and choose a model.").wrap(Wrap{trim:false}).block(block(" Connections ",true)),rows[1]);
    } else {
        selectable_list(
            frame,
            app,
            rows[1],
            "Saved connections",
            items,
            app.connection_selected,
        );
    }
    if let Some((name, p)) = app.config.profiles.iter().nth(app.connection_selected) {
        paragraph(
            frame,
            rows[2],
            vec![
                line(
                    format!(
                        "{name} · {} context tokens · {} steps per turn",
                        p.context_tokens, p.max_turns
                    ),
                    TEXT(),
                ),
                line(
                    if p.local_model.is_some() {
                        "Alt loads this model locally in CPU mode.".into()
                    } else {
                        p.endpoint.clone()
                    },
                    MUTED(),
                ),
                line(
                    format!(
                        "Authentication: {}",
                        p.api_key_env.as_deref().unwrap_or("none configured")
                    ),
                    MUTED(),
                ),
            ],
        );
    }
    buttons(
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

fn sessions(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .split(area);
    frame.render_widget(
        block(
            if app.archived {
                " Search archived conversations "
            } else {
                " Search conversations · / to type "
            },
            app.search_focus,
        ),
        rows[0],
    );
    editor(
        frame,
        inset(rows[0], 1, 1),
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
        frame.render_widget(Paragraph::new("No conversations match.\n\nStart a conversation from Home, clear the search, or switch between current and archived conversations.").wrap(Wrap{trim:false}).block(block(" Saved work ",true)),rows[1]);
    } else {
        selectable_list(
            frame,
            app,
            rows[1],
            "Saved work",
            items,
            app.session_selected,
        );
    }
    if let Some(s) = app.sessions.get(app.session_selected) {
        paragraph(
            frame,
            rows[2],
            line(
                format!(
                    "Project: {}\nResume uses the conversation's original model and connection.",
                    s.cwd
                ),
                MUTED(),
            ),
        );
    }
    buttons(
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

fn settings(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Min(4),
        Constraint::Length(4),
    ])
    .split(area);
    paragraph(
        frame,
        rows[0],
        vec![
            line("Make Alt fit your computer", TEXT()),
            line(
                "Change a setting with Enter. Changes apply to new conversations.",
                MUTED(),
            ),
        ],
    );
    // Category tabs; ←→ switches, every setting stays one keystroke away.
    let tabs = Layout::horizontal([Constraint::Percentage(20); 5]).split(rows[1]);
    for (i, label) in App::SETTINGS_TABS.iter().enumerate() {
        let active = i == app.settings_tab;
        frame.render_widget(
            Paragraph::new(format!(
                " {} ",
                if active { theme::cur().pointer } else { ' ' }
            ))
            .style(Style::default().fg(if active { ACCENT() } else { MUTED() })), // pointer cell
            tabs[i],
        );
        frame.render_widget(
            Paragraph::new((*label).to_string()).style(Style::default().fg(if active {
                ACCENT()
            } else {
                MUTED()
            })),
            Rect::new(tabs[i].x + 2, tabs[i].y, tabs[i].width.saturating_sub(2), 1),
        );
        app.hits.push((tabs[i], Hit::SettingsTab(i)));
    }
    // Every settings row in one place, indexed exactly as setting_action maps.
    let context = app
        .current_profile()
        .map(|(_, p)| p.context_tokens)
        .unwrap_or(app.preferences.context_tokens);
    let budget = app
        .current_profile()
        .map(|(_, p)| p.max_turns)
        .unwrap_or(app.preferences.max_turns);
    let all = app.settings_all_items(context, budget);
    let indexes = App::settings_tab_indexes(app.settings_tab);
    let items: Vec<(String, String)> = indexes.iter().map(|i| all[*i].clone()).collect();
    selectable_list(
        frame,
        app,
        rows[2],
        "Preferences",
        items,
        app.settings_selected,
    );
    paragraph(
        frame,
        rows[3],
        vec![
            line(
                format!(
                    "Detected: {} RAM · {} available · {} CPU threads",
                    human_bytes(app.hardware.ram),
                    human_bytes(app.hardware.available_ram),
                    app.hardware.threads
                ),
                MUTED(),
            ),
            line(app.hardware.gpu.clone(), MUTED()),
            line(
                "CPU is the default. GPU layers require a compatible llama-server build.",
                MUTED(),
            ),
            line(format!("Data: {}", app.root.display()), MUTED()),
        ],
    );
}

fn help(frame: &mut Frame, area: Rect, scroll: u16) {
    let text = vec![
        line("YOU DO NOT NEED TO KNOW HOW TO CODE", ACCENT()),
        line(
            "Tell Alt the result you want. It can inspect your files, explain a plan, make changes, and run checks. Review what it did; models can make mistakes.",
            TEXT(),
        ),
        line("", TEXT()),
        line("1. CONNECT A MODEL", ACCENT()),
        line(
            "A connection points to the application running your model. Use Ollama, LM Studio, another compatible server, or a local GGUF model file. The guided setup checks the address and lists models.",
            TEXT(),
        ),
        line("", TEXT()),
        line("2. CHOOSE A PROJECT", ACCENT()),
        line(
            "Pick the folder containing the files you want to work on. The project brief keeps your goals and decisions handy. It is included with your messages.",
            TEXT(),
        ),
        line("", TEXT()),
        line("3. DESCRIBE THE JOB AND REVIEW ACTIONS", ACCENT()),
        line(
            "Use ordinary language. Activity shows commands, edits, and actual results. Read each approval, then reject it, allow it once, or trust the session. Guided changes checkpoints edits and isolates configured checks. Full access enables arbitrary terminal commands with your OS access. Task progress shows diffs, undo and fresh check evidence.",
            TEXT(),
        ),
        line("", TEXT()),
        line("FIND YOUR WAY", ACCENT()),
        line(
            "Click a page or press Alt+1…8. Tab focuses navigation. Ctrl+P opens quick actions. Enter sends a message; Alt+Enter or Ctrl+J adds a new line. Paste works. Esc stops a task. Ctrl+Q exits.",
            TEXT(),
        ),
        line("", TEXT()),
        line("WHEN SOMETHING GOES WRONG", ACCENT()),
        line(
            "Your conversation stays saved. Check the connection address, model application, available memory, and tool evidence. Reconnect a saved conversation to continue. Paused model downloads resume when you choose the same file.",
            TEXT(),
        ),
    ];
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
            .block(block(" A quick guide · PgUp/PgDn to scroll ", false)),
        area,
    );
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(4));
    let height = height.min(area.height.saturating_sub(2));
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn editor(frame: &mut Frame, area: Rect, editor: &Editor, multiline: bool, focused: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    if !multiline {
        let (text, column) = editor.line_view(area.width as usize);
        frame.render_widget(Paragraph::new(text).style(style(TEXT())), area);
        if focused {
            frame.set_cursor_position((area.x + column.min(area.width - 1), area.y));
        }
    } else {
        use unicode_width::UnicodeWidthStr;
        let before = &editor.text[..editor.cursor];
        let row = before.lines().count().saturating_sub(1) + usize::from(before.ends_with('\n'));
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
                .style(style(TEXT())),
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

fn modal(frame: &mut Frame, app: &mut App, area: Rect) {
    if !app.permissions.is_empty() {
        permission_modal(frame, app, area);
        return;
    }
    let Some(dialog) = app.dialog.clone() else {
        return;
    };
    let rect = centered(
        area,
        84,
        match dialog {
            Dialog::Connection(_) => 20,
            Dialog::Confirm { .. } => 20,
            Dialog::Input {
                multiline: true, ..
            } => 26,
            Dialog::Palette { .. } => 20,
            _ => 24,
        },
    );
    app.motion.dialog_rect = Some(rect);
    frame.render_widget(Clear, rect);
    match dialog {
        Dialog::Menu {
            title,
            description,
            items,
            selected,
        } => {
            frame.render_widget(block(format!(" {title} "), true), rect);
            let rows = Layout::vertical([
                Constraint::Length(if description.lines().count() > 1 {
                    7
                } else {
                    3
                }),
                Constraint::Min(4),
                Constraint::Length(1),
            ])
            .split(inset(rect, 2, 1));
            paragraph(
                frame,
                rows[0],
                description
                    .lines()
                    .map(|s| line(s.to_owned(), MUTED()))
                    .collect::<Vec<_>>(),
            );
            let labels = items
                .iter()
                .map(|(label, hint, _)| (label.clone(), hint.clone()))
                .collect();
            selectable_list(frame, app, rows[1], "Choose an option", labels, selected);
            // Menu rows select and activate using the same mouse convention as buttons.
            for (_, hit) in &mut app.hits {
                if let Hit::Row(index) = hit {
                    *hit = Hit::Choice(*index);
                }
            }
            buttons(
                frame,
                app,
                rows[2],
                &[("Choose", "modal-submit"), ("Cancel", "modal-cancel")],
            );
        }
        Dialog::Palette { query, selected } => {
            let th = theme::cur();
            use unicode_width::UnicodeWidthStr;
            frame.render_widget(block(" Command palette ", true), rect);
            let rows = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(4),
                Constraint::Length(1),
            ])
            .split(inset(rect, 2, 1));
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("{} ", th.pointer), style(ACCENT())),
                    Span::styled(query.text.clone(), style(TEXT())),
                ]))
                .block(
                    Block::default()
                        .borders(Borders::BOTTOM)
                        .border_style(style(BORDER())),
                ),
                rows[0],
            );
            frame.set_cursor_position((rows[0].x + 2 + query.text.width() as u16, rows[0].y));
            let items = app.palette_filtered();
            let inner = inset(rows[1], 0, 0);
            let visible = (inner.height / 2) as usize;
            let start = (selected / visible.max(1) * visible.max(1)).min(items.len());
            let mut lines = Vec::new();
            for (offset, (label, hint, _)) in items.iter().enumerate().skip(start).take(visible) {
                let active = offset == selected;
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} ", if active { th.pointer } else { ' ' }),
                        style(if active { ACCENT() } else { BORDER() }),
                    ),
                    Span::styled(
                        label.clone(),
                        if active {
                            style(ACCENT()).add_modifier(Modifier::BOLD)
                        } else {
                            style(TEXT())
                        },
                    ),
                ]));
                lines.push(line(format!("    {hint}"), MUTED()));
            }
            frame.render_widget(
                Paragraph::new(lines).style(Style::default().bg(PANEL())),
                rows[1],
            );
            for (offset, _) in items.iter().enumerate().skip(start).take(visible) {
                let y = inner.y + (offset - start) as u16 * 2;
                if y + 1 < inner.bottom() {
                    app.hits
                        .push((Rect::new(inner.x, y, inner.width, 2), Hit::Row(offset)));
                }
            }
            // Clicking a result selects and runs it, like every other list.
            for (_, hit) in &mut app.hits {
                if let Hit::Row(index) = hit {
                    *hit = Hit::Choice(*index);
                }
            }
            paragraph(
                frame,
                rows[2],
                line(
                    format!(
                        "{} result(s) · Type to filter · ↑↓ move · Enter run · Esc close",
                        items.len()
                    ),
                    MUTED(),
                ),
            );
        }
        Dialog::Connection(form) => {
            frame.render_widget(block(" Connect your model · Step 1 of 2 ", true), rect);
            let inner = inset(rect, 2, 1);
            paragraph(
                frame,
                Rect::new(inner.x, inner.y, inner.width, 3),
                vec![
                    line(
                        "Give this connection a name, then check its server address.",
                        TEXT(),
                    ),
                    line(
                        "Start the server in your model application first. No API key is needed for most local servers.",
                        MUTED(),
                    ),
                ],
            );
            for (i, label) in [
                "Connection name",
                "Server address",
                "API key variable (optional; enter a variable name, not the key)",
            ]
            .iter()
            .enumerate()
            {
                let field = Rect::new(inner.x, inner.y + 3 + i as u16 * 3, inner.width, 3);
                frame.render_widget(block(format!(" {label} "), form.focus == i), field);
                editor(
                    frame,
                    inset(field, 1, 1),
                    &form.fields[i],
                    false,
                    form.focus == i,
                );
                app.hits.push((field, Hit::Field(i)));
            }
            paragraph(
                frame,
                Rect::new(inner.x, inner.y + 12, inner.width, 2),
                line(
                    "Tab changes fields · Enter checks the connection · Alt+M types a model ID",
                    MUTED(),
                ),
            );
            buttons(
                frame,
                app,
                Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
                &[
                    ("Check connection", "test-form"),
                    ("Type model ID", "manual-model"),
                    ("Cancel", "modal-cancel"),
                ],
            );
        }
        Dialog::PickModel {
            draft,
            models,
            selected,
            query,
        } => {
            frame.render_widget(block(" Choose your model · Step 2 of 2 ", true), rect);
            let rows = Layout::vertical([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(4),
                Constraint::Length(1),
            ])
            .split(inset(rect, 2, 1));
            let shown = crate::presets::filter_models(&models, &query.text);
            paragraph(
                frame,
                rows[0],
                vec![
                    line(
                        format!("{} is connected. Choose the model to use.", draft.name),
                        GOOD(),
                    ),
                    line(
                        shown
                            .get(selected)
                            .map(|m| format!("{} — {}", m.id, m.blurb()))
                            .unwrap_or_default(),
                        MUTED(),
                    ),
                ],
            );
            paragraph(
                frame,
                rows[1],
                line(
                    format!(
                        "Search: {}▏ · {} of {} · type to filter",
                        query.text,
                        shown.len(),
                        models.len()
                    ),
                    ACCENT(),
                ),
            );
            selectable_list(
                frame,
                app,
                rows[2],
                "Available models",
                shown.iter().map(|m| (m.id.clone(), m.describe())).collect(),
                selected,
            );
            buttons(
                frame,
                app,
                rows[3],
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
            frame.render_widget(block(format!(" {title} "), true), rect);
            let rows = Layout::vertical([
                Constraint::Length(4),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(inset(rect, 2, 1));
            paragraph(frame, rows[0], line(hint, MUTED()));
            frame.render_widget(block(" Your text ", true), rows[1]);
            editor(frame, inset(rows[1], 1, 1), &input, multiline, true);
            buttons(
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
            frame.render_widget(block(format!(" {title} "), true), rect);
            let rows = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(4),
                Constraint::Length(1),
            ])
            .split(inset(rect, 2, 1));
            paragraph(
                frame,
                rows[0],
                vec![
                    line(path.display().to_string(), ACCENT()),
                    line(
                        if matches!(kind, BrowserKind::Project) {
                            "Enter opens a folder · Ctrl+S chooses the current folder"
                        } else {
                            "Enter opens folders or imports the selected file; files are not moved."
                        },
                        MUTED(),
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
            selectable_list(frame, app, rows[1], "Files and folders", labels, selected);
            let choices = if matches!(kind, BrowserKind::Project) {
                vec![
                    ("Choose this folder", "choose-folder"),
                    ("Open selected", "modal-submit"),
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
            buttons(frame, app, rows[2], &choices);
        }
        Dialog::Confirm {
            title,
            body,
            selected,
            ..
        } => {
            frame.render_widget(block(format!(" {title} "), true), rect);
            let rows = Layout::vertical([Constraint::Min(5), Constraint::Length(2)])
                .split(inset(rect, 2, 1));
            paragraph(
                frame,
                rows[0],
                body.lines()
                    .map(|s| line(s.to_owned(), TEXT()))
                    .collect::<Vec<_>>(),
            );
            let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[1]);
            for (i, label) in ["Cancel", "Continue"].iter().enumerate() {
                frame.render_widget(
                    Paragraph::new(format!("  {label}  ")).style(
                        Style::default()
                            .bg(if selected == i { ACCENT() } else { SELECT() })
                            .fg(if selected == i { BG() } else { TEXT() }),
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
            frame.render_widget(block(format!(" {title} "), true), rect);
            let rows = Layout::vertical([Constraint::Min(5), Constraint::Length(1)])
                .split(inset(rect, 2, 1));
            frame.render_widget(
                Paragraph::new(display_text(&body))
                    .style(style(TEXT()))
                    .wrap(Wrap { trim: false })
                    .scroll((scroll, 0)),
                rows[0],
            );
            buttons(
                frame,
                app,
                rows[1],
                &[("Back · PgUp/PgDn to scroll", "modal-submit")],
            );
        }
        Dialog::ToolDetails { tool, scroll } => {
            frame.render_widget(
                block(format!(" {} · {} ", tool.title, tool.status), true),
                rect,
            );
            let rows = Layout::vertical([Constraint::Min(5), Constraint::Length(1)])
                .split(inset(rect, 2, 1));
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
            buttons(frame, app, rows[1], &[("Close", "modal-submit")]);
        }
    }
}

fn permission_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some((_, params)) = app.permissions.front() else {
        return;
    };
    let tool = &params["toolCall"];
    let rect = centered(area, 94, area.height.saturating_sub(2));
    app.motion.dialog_rect = Some(rect);
    frame.render_widget(Clear, rect);
    frame.render_widget(
        block(" Review this action ", true).border_style(style(WARN())),
        rect,
    );
    let rows = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(5),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .split(inset(rect, 2, 1));
    paragraph(
        frame,
        rows[0],
        vec![
            line(
                tool["title"]
                    .as_str()
                    .unwrap_or("Your model wants to use a tool"),
                WARN(),
            ),
            line(
                format!(
                    "Project: {}",
                    app.session.as_ref().map(|s| s.cwd.as_str()).unwrap_or("")
                ),
                MUTED(),
            ),
            line(
                "Read the proposed command or change below. It runs with your account's access.",
                TEXT(),
            ),
            line(
                format!(
                    "{} request(s) waiting · PgUp/PgDn scroll details",
                    app.permissions.len()
                ),
                MUTED(),
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
            .block(block(" Proposed action / file changes ", false)),
        rows[1],
    );
    let host = crate::toolbox::ApprovalScope::of(params) == crate::toolbox::ApprovalScope::Host;
    paragraph(
        frame,
        rows[2],
        line(
            if host {
                "TERMINAL / EXTERNAL TOOL: runs with your own permissions. \"Allow terminal this session\" is separate from project trust and lasts until you reconnect. Ctrl+Y allows once; Ctrl+N rejects."
            } else {
                "Allow this session accepts future project actions (edits, checks, skill helpers) until you reconnect; terminal still asks. Ctrl+Y allows once; Ctrl+N rejects."
            },
            if host { WARN() } else { MUTED() },
        ),
    );
    let cols = Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(30),
        Constraint::Percentage(40),
    ])
    .split(rows[3]);
    let blanket = if host {
        "Allow terminal this session"
    } else {
        "Allow this session"
    };
    for (i, label) in ["Reject", "Allow once", blanket].iter().enumerate() {
        frame.render_widget(
            Paragraph::new(format!(" {label} ")).style(
                Style::default()
                    .bg(if app.permission_selected == i {
                        WARN()
                    } else {
                        SELECT()
                    })
                    .fg(if app.permission_selected == i {
                        BG()
                    } else {
                        TEXT()
                    }),
            ),
            cols[i],
        );
        app.hits.push((cols[i], Hit::Choice(i)));
    }
}

fn task(frame: &mut Frame, app: &mut App, area: Rect) {
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
            line("Inspect → Plan → Edit → Test → Results", ACCENT()),
            line(
                format!(
                    "Access: {} · {} configured check(s)",
                    app.preferences.access_policy.label(),
                    app.task_view.checks.len()
                ),
                MUTED(),
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
        summary.push(line(
            format!("Evidence: {evidence}"),
            if verification.is_some_and(|v| v.complete) {
                GOOD()
            } else {
                WARN()
            },
        ));
        summary.push(line(format!("Next: {}", t.next), TEXT()));
        summary.push(line(
            format!("Now: {} · {} tracked edit(s)", t.phase, t.changes),
            TEXT(),
        ));
        summary.push(line(
            format!("Goal: {}", crate::project::bounded(&t.goal, 180)),
            TEXT(),
        ));
    } else {
        summary.push(line(
            "Your model's file changes and check results appear here.",
            TEXT(),
        ));
        summary.push(line(
            "Choose Configure checks to tell Alt what should prove your project works.",
            TEXT(),
        ));
        summary.push(line(
            "You can run checks, inspect evidence and undo edits without asking the model.",
            MUTED(),
        ));
    }
    frame.render_widget(
        Paragraph::new(summary)
            .wrap(Wrap { trim: false })
            .block(block(" What happened · What next ", false)),
        rows[0],
    );
    if app.task_view.changes.is_empty() {
        frame.render_widget(Paragraph::new("No tracked edits yet.\n\nEdits made with Alt's edit tool save their original contents here. Full access terminal commands can have effects outside the project and are not covered by file undo.\n\nMemory and evidence remain available after restarting Alt.").wrap(Wrap{trim:false}).block(block(" Changes and checkpoints ",false)),rows[1]);
    } else {
        selectable_list(
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
    buttons(
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
    buttons(
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

fn files(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .split(area);
    paragraph(
        frame,
        rows[0],
        vec![
            line(
                format!(
                    "{} · filter: {}",
                    app.preferences.project.display(),
                    app.workbench.filter
                ),
                TEXT(),
            ),
            line(
                "Enter views · S finds text · G opens a line · E edits · N creates · D diffs",
                MUTED(),
            ),
        ],
    );
    let items = app
        .filtered_files()
        .iter()
        .map(|p| ((*p).to_string(), "Project file".into()))
        .collect();
    selectable_list(
        frame,
        app,
        rows[1],
        "Project files",
        items,
        app.workbench.file_selected,
    );
    buttons(
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
fn jobs(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(6),
        Constraint::Min(4),
        Constraint::Length(2),
    ])
    .split(area);
    paragraph(
        frame,
        rows[0],
        line(
            if app.workbench.attached {
                "TERMINAL INPUT ACTIVE · Ctrl+] detaches · Ctrl+C interrupts the program"
            } else {
                "Terminal jobs · Enter attaches · N starts · S stops · R restarts · H health · L logs"
            },
            if app.workbench.attached {
                ACCENT()
            } else {
                MUTED()
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
    selectable_list(
        frame,
        app,
        rows[1],
        "Managed jobs",
        items,
        app.workbench.job_selected,
    );
    let panel = block("Live terminal", app.workbench.attached);
    let inner = panel.inner(rows[2]);
    frame.render_widget(panel, rows[2]);
    frame.render_widget(
        Paragraph::new(display_text(&app.workbench.screen)).style(style(TEXT())),
        inner,
    );
    if app.workbench.terminal.is_some() && app.workbench.dimensions != (inner.width, inner.height)
        && app.terminal_input(serde_json::json!({"action":"resize","cols":inner.width.max(10),"rows":inner.height.max(2)})).is_ok(){app.workbench.dimensions=(inner.width,inner.height);}
    buttons(
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
fn context(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::vertical([Constraint::Min(5), Constraint::Length(2)]).split(area);
    frame.render_widget(
        Paragraph::new(display_text(&app.workbench.context))
            .block(block("Context & pinned requirements", false))
            .wrap(Wrap { trim: false })
            .scroll((app.help_scroll, 0)),
        rows[0],
    );
    buttons(
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
    #[test]
    fn meters_fill_in_eighth_cell_steps() {
        assert_eq!(meter_bar(0.0, 4), "····");
        assert_eq!(meter_bar(1.0, 4), "████");
        assert_eq!(meter_bar(0.5, 4), "██··");
        assert_eq!(meter_bar(0.5625, 4), "██▎·");
        assert_eq!(meter_bar(0.59375, 4), "██▍·");
        for step in 0..=80 {
            assert_eq!(meter_bar(step as f64 / 80.0, 10).chars().count(), 10);
        }
    }
    #[tokio::test]
    async fn motion_is_a_saved_setting_reachable_from_settings_and_palette() {
        let (dir, mut app) = setup();
        assert!(App::settings_tab_indexes(0).contains(&23));
        assert!(
            app.palette_items()
                .iter()
                .any(|(label, _, _)| label.starts_with("Setting: Motion:"))
        );
        app.manager_action("motion", serde_json::json!({})).unwrap();
        assert!(render(&mut app, 80, 24).contains("Reduced"));
        app.manager_action("motion-select", serde_json::json!({"level":"reduced"}))
            .unwrap();
        assert_eq!(app.motion.level, motion::Level::Reduced);
        let saved = std::fs::read_to_string(dir.path().join("preferences.toml")).unwrap();
        assert!(saved.contains("motion = \"reduced\""), "{saved}");
    }
    #[tokio::test]
    async fn adaptive_pacing_runs_fast_only_while_animating() {
        let (_dir, mut app) = setup();
        app.motion = motion::Motion::new(motion::Level::Full);
        app.motion.splash_until = None;
        app.motion.pin(Some(0.0));
        assert_eq!(app.motion.frame_period(), motion::IDLE_FRAME);
        app.context_used = Some(1024);
        app.context_size = Some(8192);
        render(&mut app, 100, 30);
        app.context_used = Some(6144);
        render(&mut app, 100, 30);
        assert_eq!(app.motion.frame_period(), motion::ACTIVE_FRAME);
        app.motion.pin(Some(motion::GLIDE_MS + 1.0));
        let text = render(&mut app, 100, 30);
        assert!(text.contains("6K/8K"), "{text}");
        assert_eq!(app.motion.frame_period(), motion::IDLE_FRAME);
    }
    fn animated(app: &mut App) {
        app.motion = motion::Motion::new(motion::Level::Full);
        app.motion.splash_until = None;
        app.motion.pin(Some(0.0));
    }
    #[tokio::test]
    async fn pages_slide_in_and_settle_on_the_exact_final_frame() {
        let (_dir, mut app) = setup();
        app.set_page(Page::Help);
        let still = render(&mut app, 100, 30);
        animated(&mut app);
        render(&mut app, 100, 30);
        app.set_page(Page::Sessions);
        app.motion.pin(Some(1000.0));
        let first = render(&mut app, 100, 30);
        app.motion.pin(Some(1000.0 + motion::PAGE_MS + 1.0));
        let settled = render(&mut app, 100, 30);
        assert_ne!(first, settled, "the first frame is mid-slide");
        assert!(!app.motion.active());
        app.motion.set_level(motion::Level::Off);
        assert_eq!(settled, render(&mut app, 100, 30));
        app.set_page(Page::Help);
        assert_eq!(still, render(&mut app, 100, 30), "Off draws in place");
        // Reduced fades without moving: text is in place on the first frame.
        app.motion.set_level(motion::Level::Reduced);
        app.set_page(Page::Sessions);
        assert_eq!(settled, render(&mut app, 100, 30));
        assert!(app.motion.active());
    }
    #[tokio::test]
    async fn dialogs_pop_in_cast_a_shadow_and_fade_out() {
        let (_dir, mut app) = setup();
        animated(&mut app);
        render(&mut app, 100, 30);
        app.connection_wizard();
        let opening = render(&mut app, 100, 30);
        app.motion.pin(Some(motion::MODAL_IN_MS + 1.0));
        let open = render(&mut app, 100, 30);
        assert_ne!(opening, open, "the first frame is a smaller reveal");
        assert!(open.contains("Where will your model run?"));
        let hits = app.hits.len();
        assert!(hits > 0, "dialog targets are live from the first frame");
        app.dialog = None;
        app.motion.pin(Some(motion::MODAL_IN_MS + 2.0));
        let closing = render(&mut app, 100, 30);
        assert!(
            closing.contains("Where will your model run?"),
            "closing fades out"
        );
        app.motion
            .pin(Some(motion::MODAL_IN_MS + motion::MODAL_OUT_MS + 3.0));
        let closed = render(&mut app, 100, 30);
        assert!(!closed.contains("Where will your model run?"));
        assert!(!app.motion.active());
    }
    #[tokio::test]
    async fn notices_float_in_as_toasts_and_progress_stays_in_the_status_line() {
        let (_dir, mut app) = setup();
        animated(&mut app);
        app.set_page(Page::Help);
        render(&mut app, 100, 30);
        app.notify("Theme changed to IRIS. Everything else is unchanged.");
        app.notify("Theme changed to IRIS. Everything else is unchanged.");
        app.motion.pin(Some(motion::TOAST_EDGE_MS + 1.0));
        let text = render(&mut app, 100, 30);
        assert!(text.contains("×2"), "identical notices collapse\n{text}");
        assert_eq!(app.motion.toasts[0].tone, motion::Tone::Good);
        app.progress_status("Reading project files");
        assert_eq!(app.motion.toasts.len(), 1, "progress never toasts");
        assert_eq!(app.status, "Reading project files");
        let text = render(&mut app, 100, 30);
        assert!(
            app.motion.toasts.is_empty(),
            "a newer status retires the toast"
        );
        assert!(!text.contains("×2"));
        app.notify("Theme changed to IRIS. Everything else is unchanged.");
        app.motion
            .pin(Some(motion::TOAST_MS + motion::TOAST_EDGE_MS + 1.0));
        let text = render(&mut app, 100, 30);
        assert!(!text.contains("×2"), "toasts expire");
        assert!(app.motion.toasts.is_empty());
    }
    #[tokio::test]
    async fn streaming_replies_reveal_with_a_cursor_and_history_is_whole() {
        let (_dir, mut app) = setup();
        animated(&mut app);
        app.set_page(Page::Chat);
        let reply = "Here is the plan. ".repeat(12);
        app.messages.push_back(Message {
            role: "Alt",
            text: reply.clone(),
        });
        let history = render(&mut app, 120, 40);
        assert!(
            !history.contains('▍'),
            "a reply that is not streaming shows at once"
        );
        app.busy = true;
        app.messages.push_back(Message {
            role: "Alt",
            text: reply.clone(),
        });
        app.motion.pin(Some(1000.0));
        render(&mut app, 120, 40);
        app.motion.pin(Some(1100.0));
        let partial = render(&mut app, 120, 40);
        assert!(partial.contains('▍'), "{partial}");
        app.motion.pin(Some(2000.0));
        app.busy = false;
        let done = render(&mut app, 120, 40);
        assert!(!done.contains('▍'), "{done}");
    }
    #[tokio::test]
    async fn tool_status_changes_flash_and_running_tools_spin() {
        let (_dir, mut app) = setup();
        animated(&mut app);
        app.layout = LayoutMode::Split;
        app.set_page(Page::Help);
        app.busy = true;
        app.tools.push(Tool {
            id: "t1".into(),
            title: "Read README.md".into(),
            status: "pending".into(),
            input: serde_json::Value::Null,
            content: String::new(),
        });
        render(&mut app, 140, 40);
        app.tools[0].status = "completed".into();
        app.motion.pin(Some(10.0));
        render(&mut app, 140, 40);
        assert!(app.motion.active(), "the change flashes");
        app.motion.pin(Some(10.0 + motion::FLASH_MS + 1.0));
        render(&mut app, 140, 40);
        assert!(!app.motion.active());
    }
    fn render_buffer(app: &mut App, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        terminal.backend().buffer().clone()
    }
    #[tokio::test]
    async fn splash_shows_the_wordmark_briefly_and_keys_pass_straight_through() {
        let (_dir, mut app) = setup();
        app.motion = motion::Motion::new(motion::Level::Full);
        app.motion.pin(Some(100.0));
        let text = render(&mut app, 80, 24);
        assert!(text.contains("▄▀█ █   ▀█▀"), "{text}");
        assert!(!text.contains("Start here"), "the splash covers the screen");
        app.key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::ALT))
            .unwrap();
        assert_eq!(app.page, Page::Chat, "the key still acted");
        let text = render(&mut app, 80, 24);
        assert!(!text.contains("▄▀█ █   ▀█▀"));
        assert!(app.motion.active(), "the interface cascades in");
        // Left alone, it ends by itself within its budget.
        let (_dir, mut app) = setup();
        app.motion = motion::Motion::new(motion::Level::Full);
        app.motion.pin(Some(motion::SPLASH_MS + 1.0));
        assert!(!render(&mut app, 80, 24).contains("▄▀█"));
        let (_dir, mut app) = setup();
        app.motion = motion::Motion::new(motion::Level::Reduced);
        assert!(
            !render(&mut app, 80, 24).contains("▄▀█"),
            "Reduced has no splash"
        );
    }
    #[tokio::test]
    async fn hover_glows_and_clicks_flash_the_target() {
        use crossterm::event::{MouseButton, MouseEventKind};
        let (_dir, mut app) = setup();
        animated(&mut app);
        app.set_page(Page::Help);
        render_buffer(&mut app, 100, 30);
        let (rect, _) = app
            .hits
            .iter()
            .find(|(_, hit)| matches!(hit, Hit::Nav(Page::Models)))
            .cloned()
            .unwrap();
        let plain = render_buffer(&mut app, 100, 30)[(rect.x + 2, rect.y)].bg;
        app.mouse(MouseEventKind::Moved, rect.x + 2, rect.y)
            .unwrap();
        app.motion.pin(Some(motion::HOVER_MS + 1.0));
        render_buffer(&mut app, 100, 30);
        app.motion.pin(Some(2.0 * motion::HOVER_MS + 2.0));
        let hovered = render_buffer(&mut app, 100, 30)[(rect.x + 2, rect.y)].bg;
        assert_ne!(plain, hovered, "hover tints the target");
        app.mouse(MouseEventKind::Down(MouseButton::Left), rect.x + 2, rect.y)
            .unwrap();
        assert_eq!(app.page, Page::Models);
        assert!(app.motion.press().is_some(), "the click flashes");
    }
    #[tokio::test]
    async fn focus_rings_breathe_only_while_a_task_runs_and_the_wordmark_is_a_gradient() {
        let (_dir, mut app) = setup();
        animated(&mut app);
        app.set_page(Page::Settings);
        let colors = |app: &mut App| {
            let buffer = render_buffer(app, 100, 30);
            (0..30)
                .flat_map(|y| (0..100).map(move |x| (x, y)))
                .map(|(x, y)| buffer[(x, y)].fg)
                .collect::<Vec<_>>()
        };
        app.motion.pin(Some(1000.0));
        let idle_a = colors(&mut app);
        app.motion.pin(Some(2300.0));
        let idle_b = colors(&mut app);
        assert_eq!(idle_a, idle_b, "an idle screen does not change");
        app.busy = true;
        app.motion.pin(Some(1300.0));
        let busy_a = colors(&mut app);
        app.motion.pin(Some(2600.0));
        let busy_b = colors(&mut app);
        assert_ne!(busy_a, busy_b, "the focus ring breathes while working");
        let buffer = render_buffer(&mut app, 100, 30);
        assert_ne!(
            buffer[(2, 0)].fg,
            buffer[(4, 0)].fg,
            "A and T differ in hue"
        );
    }
    /// Frame-time budget (M05): drawing with every effect active must stay far
    /// inside the 16 ms animation frame. The same scripted session (page
    /// changes, dialogs opening and closing, toasts, a gliding meter, a busy
    /// task) runs with motion Off and Full; Full is reported against Off.
    /// Debug builds are several times slower than release, so the default
    /// debug budget is generous; `ALT_FRAME_BUDGET_MS` sets an exact one.
    fn scripted_frames(level: motion::Level) -> Vec<f64> {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::load(dir.path().to_path_buf(), "goose".into(), None).unwrap();
        app.preferences.project = dir.path().to_path_buf();
        app.motion = motion::Motion::new(level);
        app.motion.splash_until = None;
        app.layout = LayoutMode::Split;
        app.busy = true;
        app.context_size = Some(8192);
        for i in 0..40 {
            app.messages.push_back(Message {
                role: if i % 2 == 0 { "You" } else { "Alt" },
                text: "A line of conversation with some length to wrap. ".repeat(4),
            });
        }
        let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
        let mut times = Vec::new();
        for frame in 0..300u32 {
            app.motion.pin(Some(frame as f64 * 16.0));
            if frame % 40 == 0 {
                app.set_page(if frame % 80 == 0 {
                    Page::Chat
                } else {
                    Page::Home
                });
                app.notify(format!("Notice {frame}"));
                app.context_used = Some(1024 + frame as u64 * 20);
            }
            match frame % 60 {
                20 => app.connection_wizard(),
                40 => app.dialog = None,
                _ => {}
            }
            let started = std::time::Instant::now();
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        times
    }
    #[test]
    fn frame_time_stays_inside_budget_with_every_effect_active() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _guard = runtime.enter();
        let off = scripted_frames(motion::Level::Off);
        let full = scripted_frames(motion::Level::Full);
        let pick = |t: &[f64], q: usize| t[t.len() * q / 100];
        eprintln!(
            "frame time at 160x48 (median / p95 ms): off {:.2} / {:.2}, full {:.2} / {:.2}",
            pick(&off, 50),
            pick(&off, 95),
            pick(&full, 50),
            pick(&full, 95)
        );
        let budget: f64 = std::env::var("ALT_FRAME_BUDGET_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(if cfg!(debug_assertions) { 60.0 } else { 8.0 });
        assert!(
            pick(&full, 95) < budget,
            "p95 {:.2} ms over the {budget} ms budget",
            pick(&full, 95)
        );
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
        app.pending_prompt = Some("Please keep my original request".into());
        app.workspace_update(crate::workspace::Update::Error("Server unavailable".into()))
            .unwrap();
        assert_eq!(app.composer.text, "Please keep my original request");
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
    fn home_is_a_card_dashboard_when_wide_and_a_list_at_80x24() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::load(dir.path().to_path_buf(), "goose".into(), None).unwrap();
        let wide = render(&mut app, 120, 40);
        assert!(wide.contains("◈ MODEL") && wide.contains("▣ PROJECT") && wide.contains("◇ TOOLS"));
        assert!(wide.contains("Try a practice project"));
        assert!(!wide.contains("Start here"));
        // Every action card is a mouse target with the list's indexes.
        let cards = app
            .hits
            .iter()
            .filter(|(_, hit)| matches!(hit, Hit::Home(_)))
            .count();
        assert_eq!(cards, 8);
        let narrow = render(&mut app, 80, 24);
        assert!(narrow.contains("Start here"));
        assert!(!narrow.contains("◈ MODEL"));
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
                    runtime_key: None,
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
