//! Buffer effects applied after widgets render: fades, slides, dims, shadows and
//! clipped reveals. They only recolour or move cells that were already drawn,
//! so layout, hit targets and text content are exactly what the widgets made.
use super::motion::blend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;
use ratatui::style::Color;

fn rgb(color: Color, fallback: Color) -> Color {
    match color {
        Color::Reset => fallback,
        other => other,
    }
}

/// Blend every cell's colours in `area` toward `toward` by `amount` (0..1).
pub fn fade(buf: &mut Buffer, area: Rect, toward: Color, amount: f64, text: Color) {
    if amount <= 0.0 {
        return;
    }
    let area = area.intersection(buf.area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.fg = blend(rgb(cell.fg, text), toward, amount);
            cell.bg = blend(rgb(cell.bg, toward), toward, amount);
        }
    }
}

/// Shift the cells of `area` right by `dx`, filling the gap with `bg`.
pub fn slide_right(buf: &mut Buffer, area: Rect, dx: u16, bg: Color) {
    let area = area.intersection(buf.area);
    if dx == 0 || area.width == 0 {
        return;
    }
    for y in area.top()..area.bottom() {
        let row: Vec<Cell> = (area.left()..area.right())
            .map(|x| buf[(x, y)].clone())
            .collect();
        for (i, x) in (area.left()..area.right()).enumerate() {
            let cell = &mut buf[(x, y)];
            if (i as u16) < dx {
                cell.reset();
                cell.bg = bg;
            } else {
                *cell = row[i - dx as usize].clone();
            }
        }
        // A wide glyph cut by the shift would leave half a character behind.
        let edge = area.left() + dx.min(area.width - 1);
        if buf[(edge, y)].symbol().is_empty() {
            buf[(edge, y)].set_symbol(" ");
        }
    }
}

pub fn snapshot(buf: &Buffer, area: Rect) -> Vec<Cell> {
    let area = area.intersection(buf.area);
    let mut cells = Vec::with_capacity(area.area() as usize);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            cells.push(buf[(x, y)].clone());
        }
    }
    cells
}

/// Grow `rect` from `scale` of its size about its centre.
pub fn scaled(rect: Rect, scale: f64) -> Rect {
    let scale = scale.clamp(0.0, 1.2);
    let w = ((rect.width as f64 * scale).round() as u16).clamp(1, rect.width);
    let h = ((rect.height as f64 * scale).round() as u16).clamp(1, rect.height);
    Rect::new(
        rect.x + (rect.width - w) / 2,
        rect.y + (rect.height - h) / 2,
        w,
        h,
    )
}

/// Keep only the part of a freshly drawn dialog inside `visible`; cells outside
/// return to what was beneath (`before`, captured over `area`).
pub fn clip_to(buf: &mut Buffer, area: Rect, before: &[Cell], dialog: Rect, visible: Rect) {
    let area = area.intersection(buf.area);
    let width = area.width as usize;
    for y in dialog.top()..dialog.bottom() {
        for x in dialog.left()..dialog.right() {
            if !area.contains((x, y).into()) || visible.contains((x, y).into()) {
                continue;
            }
            let index = (y - area.y) as usize * width + (x - area.x) as usize;
            if let Some(cell) = before.get(index) {
                buf[(x, y)] = cell.clone();
            }
        }
    }
}

/// A soft drop shadow one row below and two columns right of `rect`.
pub fn shadow(buf: &mut Buffer, rect: Rect, color: Color, amount: f64) {
    let bounds = buf.area;
    let right = Rect::new(rect.right(), rect.y + 1, 2, rect.height.saturating_sub(1));
    let below = Rect::new(rect.x + 2, rect.bottom(), rect.width, 1);
    for part in [right, below] {
        let part = part.intersection(bounds);
        for y in part.top()..part.bottom() {
            for x in part.left()..part.right() {
                let cell = &mut buf[(x, y)];
                cell.bg = blend(rgb(cell.bg, color), color, amount);
                cell.fg = blend(rgb(cell.fg, color), color, amount * 0.8);
            }
        }
    }
}

/// Fade a closed dialog's colours out over what is now beneath it. The new
/// screen's text is shown from the first frame; only the dialog's colours
/// linger, so nothing stale can be read or clicked.
pub fn fade_out(buf: &mut Buffer, rect: Rect, cells: &[Cell], t: f64, text: Color, bg: Color) {
    let area = rect.intersection(buf.area);
    if area != rect {
        return;
    }
    let mut iter = cells.iter();
    for y in rect.top()..rect.bottom() {
        for x in rect.left()..rect.right() {
            let Some(old) = iter.next() else { return };
            let cell = &mut buf[(x, y)];
            let (new_fg, new_bg) = (rgb(cell.fg, text), rgb(cell.bg, bg));
            cell.fg = blend(rgb(old.fg, text), new_fg, t);
            cell.bg = blend(rgb(old.bg, bg), new_bg, t);
        }
    }
}

/// Horizontal gradient across the foreground of `area`.
pub fn gradient(buf: &mut Buffer, area: Rect, from: Color, to: Color) {
    let area = area.intersection(buf.area);
    let span = area.width.saturating_sub(1).max(1) as f64;
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            if cell.symbol() != " " {
                cell.fg = blend(from, to, (x - area.x) as f64 / span);
            }
        }
    }
}

/// Tint the background of `area` toward `color`.
pub fn tint(buf: &mut Buffer, area: Rect, color: Color, amount: f64, bg: Color) {
    let area = area.intersection(buf.area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.bg = blend(rgb(cell.bg, bg), color, amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(text: &str) -> Buffer {
        let mut buf = Buffer::empty(Rect::new(0, 0, text.chars().count() as u16, 1));
        buf.set_string(
            0,
            0,
            text,
            ratatui::style::Style::default().bg(Color::Rgb(0, 0, 0)),
        );
        buf
    }
    fn row(buf: &Buffer) -> String {
        (0..buf.area.width).map(|x| buf[(x, 0)].symbol()).collect()
    }

    #[test]
    fn slide_moves_cells_and_clears_the_gap() {
        let mut buf = buffer("abcdef");
        let area = buf.area;
        slide_right(&mut buf, area, 2, Color::Rgb(1, 2, 3));
        assert_eq!(row(&buf), "  abcd");
        assert_eq!(buf[(0, 0)].bg, Color::Rgb(1, 2, 3));
    }

    #[test]
    fn fade_keeps_text_and_moves_colour() {
        let mut buf = buffer("hi");
        buf[(0, 0)].fg = Color::Rgb(200, 200, 200);
        let area = buf.area;
        fade(&mut buf, area, Color::Rgb(0, 0, 0), 0.5, Color::White);
        assert_eq!(row(&buf), "hi");
        assert_eq!(buf[(0, 0)].fg, Color::Rgb(100, 100, 100));
    }

    #[test]
    fn clipped_reveal_restores_the_backdrop_outside_the_visible_box() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 5));
        let before = snapshot(&buf, buf.area);
        buf.set_string(2, 1, "xxxxx", ratatui::style::Style::default());
        buf.set_string(2, 3, "yyyyy", ratatui::style::Style::default());
        let dialog = Rect::new(2, 1, 5, 3);
        let visible = scaled(dialog, 0.5);
        assert!(dialog.contains(visible.as_position()));
        let area = buf.area;
        clip_to(&mut buf, area, &before, dialog, visible);
        assert_eq!(
            buf[(2, 1)].symbol(),
            " ",
            "outside the reveal returns to the backdrop"
        );
        assert_eq!(scaled(dialog, 1.0), dialog);
    }

    #[test]
    fn fade_out_shows_the_new_text_at_once_and_fades_colours() {
        let mut buf = buffer("new");
        let mut old = buffer("old");
        for x in 0..3 {
            old[(x, 0)].bg = Color::Rgb(200, 200, 200);
        }
        let cells = snapshot(&old, old.area);
        let area = buf.area;
        fade_out(&mut buf, area, &cells, 0.25, Color::White, Color::Black);
        assert_eq!(row(&buf), "new", "no stale text, even early");
        assert_eq!(buf[(0, 0)].bg, Color::Rgb(150, 150, 150));
    }
}
