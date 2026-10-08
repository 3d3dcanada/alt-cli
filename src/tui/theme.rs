//! Semantic colors shared by every part of the interface.
//!
//! Presets include their own surfaces and status colors. Custom accents adapt
//! their brightness when needed so that a dark accent on a dark surface (or a
//! pale accent on a light surface) does not hide the focused control.

use crate::config::ThemePreset;
use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub panel: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_text: Color,
    pub border: Color,
    pub select: Color,
    pub good: Color,
    pub warn: Color,
    pub bad: Color,
}

impl Palette {
    pub fn ink_on(background: Color) -> Color {
        readable_ink(background)
    }

    pub fn for_preset(preset: ThemePreset) -> Self {
        let mut palette = match preset {
            ThemePreset::Lagoon => Self {
                bg: Color::Rgb(15, 20, 29),
                panel: Color::Rgb(22, 29, 40),
                text: Color::Rgb(233, 241, 248),
                muted: Color::Rgb(166, 182, 199),
                accent: Color::Rgb(81, 211, 202),
                accent_text: Color::Black,
                border: Color::Rgb(92, 117, 137),
                select: Color::Rgb(35, 61, 72),
                good: Color::Rgb(164, 225, 151),
                warn: Color::Rgb(248, 207, 116),
                bad: Color::Rgb(255, 161, 154),
            },
            ThemePreset::Graphite => Self {
                bg: Color::Rgb(20, 20, 23),
                panel: Color::Rgb(29, 29, 33),
                text: Color::Rgb(245, 244, 242),
                muted: Color::Rgb(183, 182, 192),
                accent: Color::Rgb(181, 196, 231),
                accent_text: Color::Black,
                border: Color::Rgb(120, 120, 132),
                select: Color::Rgb(47, 49, 59),
                good: Color::Rgb(167, 217, 173),
                warn: Color::Rgb(238, 205, 151),
                bad: Color::Rgb(245, 167, 177),
            },
            ThemePreset::Aurora => Self {
                bg: Color::Rgb(23, 16, 39),
                panel: Color::Rgb(34, 24, 52),
                text: Color::Rgb(242, 232, 255),
                muted: Color::Rgb(188, 172, 212),
                accent: Color::Rgb(210, 172, 255),
                accent_text: Color::Black,
                border: Color::Rgb(135, 102, 166),
                select: Color::Rgb(64, 40, 79),
                good: Color::Rgb(158, 224, 185),
                warn: Color::Rgb(245, 211, 153),
                bad: Color::Rgb(255, 164, 197),
            },
            ThemePreset::Ember => Self {
                bg: Color::Rgb(29, 19, 17),
                panel: Color::Rgb(41, 28, 23),
                text: Color::Rgb(255, 239, 218),
                muted: Color::Rgb(203, 177, 156),
                accent: Color::Rgb(255, 185, 119),
                accent_text: Color::Black,
                border: Color::Rgb(147, 109, 87),
                select: Color::Rgb(69, 43, 30),
                good: Color::Rgb(191, 216, 143),
                warn: Color::Rgb(255, 218, 138),
                bad: Color::Rgb(255, 164, 151),
            },
            ThemePreset::Daylight => Self {
                bg: Color::Rgb(241, 246, 250),
                panel: Color::Rgb(255, 255, 255),
                text: Color::Rgb(24, 40, 52),
                muted: Color::Rgb(69, 91, 106),
                accent: Color::Rgb(0, 104, 113),
                accent_text: Color::White,
                border: Color::Rgb(119, 141, 153),
                select: Color::Rgb(212, 228, 236),
                good: Color::Rgb(27, 106, 57),
                warn: Color::Rgb(131, 80, 0),
                bad: Color::Rgb(172, 44, 56),
            },
            ThemePreset::HighContrast => Self {
                bg: Color::Rgb(0, 0, 0),
                panel: Color::Rgb(0, 0, 0),
                text: Color::Rgb(255, 255, 255),
                muted: Color::Rgb(213, 213, 213),
                accent: Color::Rgb(0, 255, 255),
                accent_text: Color::Black,
                border: Color::Rgb(187, 187, 187),
                select: Color::Rgb(42, 42, 42),
                good: Color::Rgb(163, 255, 117),
                warn: Color::Rgb(255, 240, 0),
                bad: Color::Rgb(255, 175, 175),
            },
        };
        palette.accent_text = readable_ink(palette.accent);
        palette
    }

    /// Apply an RGB accent from the appearance preferences. Terminal-default
    /// and indexed colors have no stable RGB appearance and are left unchanged.
    pub fn with_accent(mut self, accent: Color) -> Self {
        let Color::Rgb(red, green, blue) = accent else {
            return self;
        };
        let surfaces = [self.bg, self.panel, self.select].map(luminance);
        let readable = |color| {
            let foreground = luminance(color);
            surfaces
                .iter()
                .all(|background| contrast(foreground, *background) >= 4.5)
        };
        self.accent = accent;
        if !readable(accent) {
            // Preset surfaces are uniformly dark or uniformly light. Move
            // toward whichever endpoint has the strongest worst-case contrast.
            let weakest_contrast = |foreground| {
                surfaces
                    .iter()
                    .map(|background| contrast(foreground, *background))
                    .fold(f64::INFINITY, f64::min)
            };
            let endpoint: u8 = if weakest_contrast(1.0) >= weakest_contrast(0.0) {
                255
            } else {
                0
            };
            for amount in 1..=255u32 {
                let blend = |channel: u8| {
                    ((u32::from(channel) * (255 - amount) + u32::from(endpoint) * amount) / 255)
                        as u8
                };
                let adjusted = Color::Rgb(blend(red), blend(green), blend(blue));
                if readable(adjusted) {
                    self.accent = adjusted;
                    break;
                }
            }
        }
        self.accent_text = readable_ink(self.accent);
        self
    }
}

fn readable_ink(background: Color) -> Color {
    let background = luminance(background);
    if contrast(background, 0.0) >= contrast(background, 1.0) {
        Color::Rgb(0, 0, 0)
    } else {
        Color::Rgb(255, 255, 255)
    }
}

fn luminance(color: Color) -> f64 {
    let (red, green, blue) = match color {
        Color::Rgb(red, green, blue) => (red, green, blue),
        Color::White => (255, 255, 255),
        _ => (0, 0, 0),
    };
    let linear = |channel: u8| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    linear(red) * 0.2126 + linear(green) * 0.7152 + linear(blue) * 0.0722
}

fn contrast(first: f64, second: f64) -> f64 {
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRESETS: [ThemePreset; 6] = [
        ThemePreset::Lagoon,
        ThemePreset::Graphite,
        ThemePreset::Aurora,
        ThemePreset::Ember,
        ThemePreset::Daylight,
        ThemePreset::HighContrast,
    ];

    #[test]
    fn every_preset_keeps_body_hints_and_status_readable_on_all_surfaces() {
        for preset in PRESETS {
            let palette = Palette::for_preset(preset);
            for foreground in [
                palette.text,
                palette.muted,
                palette.accent,
                palette.good,
                palette.warn,
                palette.bad,
            ] {
                for background in [palette.bg, palette.panel, palette.select] {
                    let ratio = contrast(luminance(foreground), luminance(background));
                    assert!(
                        ratio >= 4.5,
                        "{preset:?}: {foreground:?} on {background:?} has contrast {ratio}"
                    );
                    if preset == ThemePreset::HighContrast {
                        assert!(ratio >= 7.0, "High Contrast text should reach 7:1");
                    }
                }
            }
            for background in [palette.bg, palette.panel] {
                assert!(
                    contrast(luminance(palette.border), luminance(background)) >= 3.0,
                    "{preset:?}: control boundaries should remain distinguishable"
                );
            }
            assert!(contrast(luminance(palette.accent), luminance(palette.accent_text)) >= 4.5);
        }
    }

    #[test]
    fn custom_accents_and_their_button_labels_stay_readable() {
        for preset in PRESETS {
            for red in [0, 64, 128, 192, 255] {
                for green in [0, 64, 128, 192, 255] {
                    for blue in [0, 64, 128, 192, 255] {
                        let requested = Color::Rgb(red, green, blue);
                        let original = Palette::for_preset(preset);
                        let adjusted = original.with_accent(requested);
                        let originally_readable = [original.bg, original.panel, original.select]
                            .iter()
                            .all(|background| {
                                contrast(luminance(requested), luminance(*background)) >= 4.5
                            });
                        if originally_readable {
                            assert_eq!(adjusted.accent, requested);
                        }
                        for background in [adjusted.bg, adjusted.panel, adjusted.select] {
                            assert!(
                                contrast(luminance(adjusted.accent), luminance(background)) >= 4.5,
                                "{preset:?}: requested {requested:?}, got {:?}",
                                adjusted.accent
                            );
                        }
                        assert!(
                            contrast(luminance(adjusted.accent), luminance(adjusted.accent_text))
                                >= 4.5
                        );
                        assert_eq!(adjusted.good, original.good);
                        assert_eq!(adjusted.warn, original.warn);
                        assert_eq!(adjusted.bad, original.bad);
                    }
                }
            }
        }
    }
}
