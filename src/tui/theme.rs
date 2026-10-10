//! Selectable interface themes. The presentation is data, not constants:
//! the active theme is installed once per frame and read through `cur()`,
//! so Settings changes it live without a restart.
use ratatui::style::Color;
use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub panel: Color,
    pub text: Color,
    pub muted: Color,
    /// Primary accent; used for focus, headers and activity.
    pub accent: Color,
    /// Secondary accent; used with `accent` for two-tone highlights.
    pub accent2: Color,
    pub border: Color,
    /// Border color for the focused panel; a "glow" ring in the neon themes.
    pub border_focused: Color,
    pub select: Color,
    pub good: Color,
    pub warn: Color,
    pub bad: Color,
    /// List pointer glyph.
    pub pointer: char,
    pub led_on: char,
    pub led_off: char,
    /// Braille animation frames for the busy indicator.
    pub spinner: &'static [&'static str],
    pub blurb: &'static str,
}

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub const THEMES: &[Theme] = &[
    Theme {
        name: "Neon Grid",
        blurb: "Electric cyan and magenta on deep space blue",
        bg: Color::Rgb(9, 12, 26),
        panel: Color::Rgb(16, 22, 40),
        text: Color::Rgb(214, 226, 244),
        muted: Color::Rgb(124, 144, 178),
        accent: Color::Rgb(0, 255, 209),
        accent2: Color::Rgb(255, 61, 176),
        border: Color::Rgb(38, 54, 84),
        border_focused: Color::Rgb(0, 255, 209),
        select: Color::Rgb(24, 44, 66),
        good: Color::Rgb(63, 255, 138),
        warn: Color::Rgb(255, 176, 0),
        bad: Color::Rgb(255, 64, 96),
        pointer: '▸',
        led_on: '●',
        led_off: '○',
        spinner: SPINNER,
    },
    Theme {
        name: "Amber CRT",
        blurb: "Warm amber phosphor on black; terminal-relic look",
        bg: Color::Rgb(12, 8, 4),
        panel: Color::Rgb(24, 16, 8),
        text: Color::Rgb(255, 196, 110),
        muted: Color::Rgb(190, 120, 51),
        accent: Color::Rgb(255, 168, 18),
        accent2: Color::Rgb(255, 214, 92),
        border: Color::Rgb(96, 60, 20),
        border_focused: Color::Rgb(255, 168, 18),
        select: Color::Rgb(64, 40, 10),
        good: Color::Rgb(255, 214, 92),
        warn: Color::Rgb(255, 196, 110),
        bad: Color::Rgb(255, 96, 32),
        pointer: '»',
        led_on: '●',
        led_off: '·',
        spinner: &["|", "/", "-", "\\"],
    },
    Theme {
        name: "IRIS",
        blurb: "Violet and lime on deep night purple",
        bg: Color::Rgb(14, 10, 24),
        panel: Color::Rgb(24, 17, 40),
        text: Color::Rgb(226, 220, 246),
        muted: Color::Rgb(150, 138, 185),
        accent: Color::Rgb(170, 128, 255),
        accent2: Color::Rgb(220, 254, 108),
        border: Color::Rgb(56, 44, 92),
        border_focused: Color::Rgb(170, 128, 255),
        select: Color::Rgb(44, 32, 78),
        good: Color::Rgb(163, 255, 190),
        warn: Color::Rgb(255, 200, 108),
        bad: Color::Rgb(255, 108, 156),
        pointer: '❯',
        led_on: '●',
        led_off: '○',
        spinner: SPINNER,
    },
    Theme {
        name: "Bleached",
        blurb: "Light theme: ink on paper with blue and orange accents",
        bg: Color::Rgb(248, 249, 252),
        panel: Color::Rgb(255, 255, 255),
        text: Color::Rgb(24, 29, 39),
        muted: Color::Rgb(106, 119, 140),
        accent: Color::Rgb(0, 120, 212),
        accent2: Color::Rgb(255, 106, 19),
        border: Color::Rgb(212, 218, 230),
        border_focused: Color::Rgb(0, 120, 212),
        select: Color::Rgb(224, 236, 252),
        good: Color::Rgb(22, 128, 61),
        warn: Color::Rgb(153, 101, 12),
        bad: Color::Rgb(200, 36, 46),
        pointer: '›',
        led_on: '●',
        led_off: '○',
        spinner: SPINNER,
    },
    // The exact pre-theme palette, kept so the previous look is one choice away.
    Theme {
        name: "Stock",
        blurb: "The previous Alt palette, unchanged",
        bg: Color::Rgb(15, 20, 29),
        panel: Color::Rgb(22, 29, 40),
        text: Color::Rgb(224, 231, 239),
        muted: Color::Rgb(147, 163, 183),
        accent: Color::Rgb(81, 211, 202),
        accent2: Color::Rgb(81, 211, 202),
        border: Color::Rgb(55, 69, 86),
        border_focused: Color::Rgb(81, 211, 202),
        select: Color::Rgb(35, 61, 72),
        good: Color::Rgb(153, 215, 143),
        warn: Color::Rgb(239, 194, 105),
        bad: Color::Rgb(246, 139, 136),
        pointer: '›',
        led_on: '●',
        led_off: '○',
        spinner: &["·"],
    },
];

pub const DEFAULT: &str = "Neon Grid";

pub fn by_name(name: &str) -> &'static Theme {
    THEMES
        .iter()
        .find(|theme| theme.name.eq_ignore_ascii_case(name))
        .unwrap_or(&THEMES[0])
}

// ponytail: the theme is installed per frame into a thread-local instead of
// threading &Theme through every helper signature; rendering is single-threaded
// per draw. If drawing ever becomes concurrent, pass the theme explicitly.
thread_local! {
    static CURRENT: RefCell<Theme> = const { RefCell::new(THEMES[0]) };
}

pub fn set(theme: Theme) {
    CURRENT.with(|current| *current.borrow_mut() = theme);
}

pub fn cur() -> Theme {
    CURRENT.with(|current| *current.borrow())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn themes_have_unique_names_and_lookup_falls_back_to_default() {
        for window in THEMES.windows(2) {
            assert_ne!(window[0].name, window[1].name);
        }
        assert_eq!(by_name(DEFAULT).name, DEFAULT);
        assert_eq!(by_name("no-such-theme").name, DEFAULT);
        assert_eq!(by_name("stock").name, "Stock");
        assert!(THEMES.len() >= 5);
    }
}
