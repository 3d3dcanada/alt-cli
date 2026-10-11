//! Motion: one clock, easing curves, keyed tweens and the transition state the
//! renderer reads each frame. Every animation is a pure function of time, so a
//! frame drawn twice at the same instant is identical and tests can pin the
//! clock. Motion never delays input or hides state: transitions are short,
//! text is never withheld, and `Off` renders every frame in its final state.
use ratatui::layout::Rect;
use ratatui::style::Color;
use std::cell::Cell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How much the interface moves. Saved as `preferences.motion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Level {
    /// Slides, pops, glides, glow and the startup splash.
    #[default]
    Full,
    /// Fades and colour changes only; nothing slides, scales or breathes.
    Reduced,
    /// Every frame is drawn in its final state.
    Off,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Full, Level::Reduced, Level::Off];
    pub fn saved_name(self) -> &'static str {
        match self {
            Level::Full => "full",
            Level::Reduced => "reduced",
            Level::Off => "off",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Level::Full => "Full",
            Level::Reduced => "Reduced",
            Level::Off => "Off",
        }
    }
    pub fn describe(self) -> &'static str {
        match self {
            Level::Full => "Slides, pops, gliding meters, focus glow and a short splash.",
            Level::Reduced => "Fades and colour changes only; nothing slides or scales.",
            Level::Off => "No animation. Every frame is drawn in its final state.",
        }
    }
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "reduced" | "reduce" | "minimal" => Level::Reduced,
            "off" | "none" | "0" | "false" => Level::Off,
            _ => Level::Full,
        }
    }
    /// The saved preference, overridden by `ALT_MOTION`. Terminals that cannot
    /// show colour blends (the Linux console, `dumb`) and `NO_MOTION` turn it off.
    /// Unit tests render in the final state unless they opt in.
    pub fn resolve(saved: &str) -> Self {
        Self::resolve_with(
            saved,
            std::env::var("ALT_MOTION").ok().as_deref(),
            std::env::var("TERM").ok().as_deref(),
            std::env::var_os("NO_MOTION").is_some(),
        )
    }
    pub fn resolve_with(
        saved: &str,
        env: Option<&str>,
        term: Option<&str>,
        no_motion: bool,
    ) -> Self {
        if let Some(value) = env.filter(|v| !v.trim().is_empty()) {
            return Self::from_name(value);
        }
        if no_motion || matches!(term, Some("linux" | "dumb")) || cfg!(test) {
            return Level::Off;
        }
        Self::from_name(saved)
    }
    pub fn moves(self) -> bool {
        self == Level::Full
    }
    pub fn fades(self) -> bool {
        self != Level::Off
    }
}

pub fn ease_out_cubic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}
pub fn ease_in_out_cubic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}
/// Overshoots slightly before settling: the "pop" used for dialogs.
pub fn ease_out_back(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Quantise progress into `steps` equal stages. Whole-area effects repaint
/// every cell they touch, so a transition drawn in a few distinct stages costs
/// a few repaints (identical frames diff to nothing) instead of one per 16 ms
/// frame — kind to slow SSH links and terminals that read output lazily.
pub fn stepped(t: f64, steps: u32) -> f64 {
    let steps = steps.max(1) as f64;
    ((t.clamp(0.0, 1.0) * steps).floor() / steps).min(1.0)
}

/// Linear blend of two colours; `t = 0` is `a`, `t = 1` is `b`. Named and
/// indexed colours cannot be mixed, so they switch at the midpoint.
pub fn blend(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let mix = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
            Color::Rgb(mix(r1, r2), mix(g1, g2), mix(b1, b2))
        }
        _ if t < 0.5 => a,
        _ => b,
    }
}

/// Durations, in milliseconds, shared by the renderer and the docs.
pub const PAGE_MS: f64 = 190.0;
pub const MODAL_IN_MS: f64 = 170.0;
pub const MODAL_OUT_MS: f64 = 110.0;
pub const GLIDE_MS: f64 = 420.0;
pub const TOAST_MS: f64 = 3200.0;
pub const TOAST_EDGE_MS: f64 = 220.0;
pub const FLASH_MS: f64 = 650.0;
pub const PRESS_MS: f64 = 140.0;
pub const HOVER_MS: f64 = 120.0;
pub const SPLASH_MS: f64 = 720.0;
/// Frame period while something is animating, and while idle.
pub const ACTIVE_FRAME: Duration = Duration::from_millis(16);
pub const IDLE_FRAME: Duration = Duration::from_millis(60);

#[derive(Debug, Clone, Copy)]
struct Tween {
    from: f64,
    to: f64,
    start: f64,
    duration: f64,
}

impl Tween {
    fn value(&self, now: f64) -> f64 {
        let t = ((now - self.start) / self.duration).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * ease_out_cubic(t)
    }
    fn done(&self, now: f64) -> bool {
        now >= self.start + self.duration
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tone {
    Info,
    Good,
    Bad,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub text: String,
    pub tone: Tone,
    pub born: f64,
    /// Identical consecutive notices collapse into one card with a count.
    pub count: u32,
}

/// Cells of the last dialog drawn, kept so a closing dialog can fade out.
#[derive(Debug, Clone)]
pub struct Retained {
    pub rect: Rect,
    pub cells: Vec<ratatui::buffer::Cell>,
}

#[derive(Debug)]
pub struct Motion {
    pub level: Level,
    epoch: Instant,
    /// Pinned clock for tests and frame-by-frame captures (milliseconds).
    pinned: Option<f64>,
    now: f64,
    tweens: HashMap<&'static str, Tween>,
    pub page_since: Option<f64>,
    pub last_page: Option<u8>,
    pub modal_since: Option<f64>,
    pub modal_key: Option<String>,
    pub modal_closing: Option<(f64, Retained)>,
    pub modal_retained: Option<Retained>,
    pub toasts: Vec<Toast>,
    /// Characters shown of a streaming reply: (message index, shown, last frame ms).
    reveal: Option<(usize, f64, f64)>,
    /// Tool id → (last seen status, flash start).
    flashes: HashMap<String, (String, Option<f64>)>,
    pub hover: Option<(u16, u16)>,
    hover_rect: Option<(Rect, f64)>,
    pub press: Option<(Rect, f64)>,
    pub splash_until: Option<f64>,
}

impl Default for Motion {
    fn default() -> Self {
        Self::new(Level::Off)
    }
}

impl Motion {
    pub fn new(level: Level) -> Self {
        Self {
            level,
            epoch: Instant::now(),
            pinned: None,
            now: 0.0,
            tweens: HashMap::new(),
            page_since: None,
            last_page: None,
            modal_since: None,
            modal_key: None,
            modal_closing: None,
            modal_retained: None,
            toasts: Vec::new(),
            reveal: None,
            flashes: HashMap::new(),
            hover: None,
            hover_rect: None,
            press: None,
            splash_until: level.moves().then_some(SPLASH_MS),
        }
    }
    /// Pin the clock (milliseconds since start); `None` returns to real time.
    #[cfg(test)]
    pub fn pin(&mut self, ms: Option<f64>) {
        self.pinned = ms;
        self.tick(false);
    }
    pub fn set_level(&mut self, level: Level) {
        self.level = level;
        if !level.moves() {
            self.splash_until = None;
        }
        if !level.fades() {
            self.page_since = None;
            self.modal_since = None;
            self.modal_closing = None;
            self.tweens.clear();
            self.reveal = None;
            self.press = None;
            self.hover_rect = None;
        }
    }
    /// Advance the clock; called once at the start of every frame. `busy`
    /// lets focus rings breathe while a task runs (and stay still otherwise).
    pub fn tick(&mut self, busy: bool) -> f64 {
        self.now = self
            .pinned
            .unwrap_or_else(|| self.epoch.elapsed().as_secs_f64() * 1000.0);
        self.toasts.retain(|t| self.now - t.born < TOAST_MS);
        // When the splash ends the interface cascades in like a page change.
        if self.splash_until.is_some_and(|until| self.now >= until) {
            self.splash_until = None;
            self.page_since = Some(self.now);
        }
        FRAME.with(|f| {
            f.set(Shared {
                now: self.now,
                level: self.level,
                busy,
            })
        });
        self.now
    }
    pub fn now(&self) -> f64 {
        self.now
    }
    /// Progress `0..=1` of an animation that began at `since` and lasts `ms`.
    pub fn progress(&self, since: Option<f64>, ms: f64) -> Option<f64> {
        let since = since?;
        let t = (self.now - since) / ms;
        (t < 1.0).then(|| t.max(0.0))
    }
    /// Glide a value toward `target`; returns the value to draw this frame.
    pub fn glide(&mut self, key: &'static str, target: f64) -> f64 {
        if !self.level.moves() {
            self.tweens.remove(key);
            return target;
        }
        let now = self.now;
        match self.tweens.get(key).copied() {
            Some(tween) if (tween.to - target).abs() < f64::EPSILON => tween.value(now),
            Some(tween) => {
                let from = tween.value(now);
                self.tweens.insert(
                    key,
                    Tween {
                        from,
                        to: target,
                        start: now,
                        duration: GLIDE_MS,
                    },
                );
                from
            }
            None => {
                // The first value is shown as is; only changes glide.
                self.tweens.insert(
                    key,
                    Tween {
                        from: target,
                        to: target,
                        start: now - GLIDE_MS,
                        duration: GLIDE_MS,
                    },
                );
                target
            }
        }
    }
    /// Note the page drawn this frame; a change starts the page transition.
    pub fn page(&mut self, page: u8) {
        if self.last_page.is_some_and(|p| p != page) && self.level.fades() {
            self.page_since = Some(self.now);
        }
        self.last_page = Some(page);
    }
    /// Note the dialog drawn this frame (`None` when none). Returns the progress
    /// of its entrance, if still running.
    pub fn modal(&mut self, key: Option<String>) -> Option<f64> {
        if key != self.modal_key {
            if key.is_some() && self.level.fades() {
                self.modal_since = Some(self.now);
                self.modal_closing = None;
            } else if key.is_none() {
                self.modal_since = None;
                if let Some(retained) = self.modal_retained.take()
                    && self.level.fades()
                {
                    self.modal_closing = Some((self.now, retained));
                }
            }
            self.modal_key = key;
        }
        if self.modal_key.is_none() {
            self.modal_retained = None;
        }
        self.progress(self.modal_since, MODAL_IN_MS)
    }
    pub fn closing(&mut self) -> Option<(f64, Retained)> {
        let (since, retained) = self.modal_closing.clone()?;
        match self.progress(Some(since), MODAL_OUT_MS) {
            Some(t) => Some((t, retained)),
            None => {
                self.modal_closing = None;
                None
            }
        }
    }
    pub fn toast(&mut self, text: &str, tone: Tone) {
        let text = text.trim();
        if text.is_empty() || !self.level.fades() {
            return;
        }
        if let Some(last) = self.toasts.last_mut()
            && last.text == text
        {
            last.count += 1;
            last.born = self.now;
            return;
        }
        // One notice at a time, like the status line it echoes: a new notice
        // replaces the old one at once, so nothing stale lingers on screen.
        self.toasts.clear();
        self.toasts.push(Toast {
            text: text.to_owned(),
            tone,
            born: self.now,
            count: 1,
        });
    }
    /// How many characters of message `index` (of `len`) to show. Only replies
    /// that grow while visible are revealed; history appears at once.
    pub fn reveal(&mut self, index: usize, len: usize, streaming: bool) -> usize {
        if !self.level.fades() {
            self.reveal = None;
            return len;
        }
        let now = self.now;
        let (shown, last) = match self.reveal {
            Some((i, shown, last)) if i == index => (shown, last),
            _ if streaming => (0.0, now),
            _ => {
                self.reveal = Some((index, len as f64, now));
                return len;
            }
        };
        let behind = len as f64 - shown;
        // Never fall more than ~a quarter second behind the model.
        let rate = (behind * 4.0).max(120.0) / 1000.0;
        let next = (shown + rate * (now - last).max(0.0)).min(len as f64);
        self.reveal = Some((index, next, now));
        next.floor() as usize
    }
    /// Colour flash for a tool whose status just changed: progress `0..1`.
    pub fn flash(&mut self, id: &str, status: &str) -> Option<f64> {
        let now = self.now;
        let fades = self.level.fades();
        let entry = self
            .flashes
            .entry(id.to_owned())
            .or_insert_with(|| (status.to_owned(), None));
        if entry.0 != status {
            entry.0 = status.to_owned();
            entry.1 = fades.then_some(now);
        }
        let since = entry.1;
        self.progress(since, FLASH_MS)
    }
    /// Hover highlight strength `0..=1` for the target under the pointer.
    pub fn hover(&mut self, rect: Option<Rect>) -> Option<(Rect, f64)> {
        if !self.level.fades() {
            return rect.map(|r| (r, 1.0));
        }
        let rect = rect?;
        let since = match self.hover_rect {
            Some((r, since)) if r == rect => since,
            _ => self.now,
        };
        self.hover_rect = Some((rect, since));
        Some((rect, ((self.now - since) / HOVER_MS).clamp(0.0, 1.0)))
    }
    pub fn pressed(&mut self, rect: Rect) {
        if self.level.fades() {
            self.press = Some((rect, self.now));
        }
    }
    pub fn press(&self) -> Option<(Rect, f64)> {
        let (rect, since) = self.press?;
        self.progress(Some(since), PRESS_MS).map(|t| (rect, t))
    }
    pub fn splash(&self) -> Option<f64> {
        let until = self.splash_until?;
        (self.now < until).then(|| self.now / until)
    }
    /// Any key or click ends the splash at once; it never holds input.
    pub fn dismiss_splash(&mut self) {
        if self.splash_until.take().is_some() && self.level.moves() {
            self.page_since = Some(self.now);
        }
    }
    /// True while something moves fast enough to need the 16 ms frame.
    pub fn active(&self) -> bool {
        let now = self.now;
        let running = |since: Option<f64>, ms: f64| since.is_some_and(|s| now - s < ms);
        running(self.page_since, PAGE_MS)
            || running(self.modal_since, MODAL_IN_MS)
            || self.modal_closing.is_some()
            || self.splash().is_some()
            || self.press().is_some()
            || self.tweens.values().any(|t| !t.done(now))
            || self
                .toasts
                .iter()
                .any(|t| now - t.born < TOAST_EDGE_MS || now - t.born > TOAST_MS - TOAST_EDGE_MS)
            || self
                .flashes
                .values()
                .any(|(_, since)| running(*since, FLASH_MS))
            || self.reveal.is_some_and(|(_, shown, _)| shown.fract() > 0.0)
            || self
                .hover_rect
                .is_some_and(|(_, since)| now - since < HOVER_MS)
    }
    pub fn frame_period(&self) -> Duration {
        if self.active() {
            ACTIVE_FRAME
        } else {
            IDLE_FRAME
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Shared {
    now: f64,
    level: Level,
    busy: bool,
}

thread_local! {
    static FRAME: Cell<Shared> = const {
        Cell::new(Shared {
            now: 0.0,
            level: Level::Off,
            busy: false,
        })
    };
}

/// Milliseconds on the motion clock for the frame being drawn.
pub fn now() -> f64 {
    FRAME.with(|f| f.get().now)
}
/// Slow 0..1 "breath" for focus rings while a task runs; flat when idle or
/// when motion is reduced or off, so an idle Alt writes nothing to the
/// terminal. Eight levels keep the ring to a few small repaints a second.
pub fn breath() -> f64 {
    let shared = FRAME.with(|f| f.get());
    if !shared.level.moves() || !shared.busy {
        return 0.0;
    }
    let phase = shared.now / 2600.0 * std::f64::consts::TAU;
    stepped((1.0 - phase.cos()) / 2.0, 8)
}
/// Spinner frame from the clock, so speed does not depend on frame rate.
pub fn spin(frames: usize) -> usize {
    ((now() / 90.0) as usize) % frames.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_curves_start_at_zero_and_settle_at_one() {
        for curve in [ease_out_cubic, ease_in_out_cubic, ease_out_back] {
            assert!(curve(0.0).abs() < 1e-9);
            assert!((curve(1.0) - 1.0).abs() < 1e-9);
        }
        assert!(ease_out_back(0.7) > 1.0, "the pop overshoots");
        assert!(ease_out_cubic(0.5) > 0.5, "ease-out leads");
    }

    #[test]
    fn stepping_limits_distinct_frames() {
        let stages: std::collections::BTreeSet<u64> = (0..=100)
            .map(|i| (stepped(ease_out_cubic(i as f64 / 100.0), 5) * 1000.0) as u64)
            .collect();
        assert_eq!(stages.len(), 6);
        assert_eq!(stepped(0.0, 5), 0.0);
        assert_eq!(stepped(1.0, 5), 1.0);
    }

    #[test]
    fn blending_mixes_rgb_and_switches_named_colours() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(200, 100, 50);
        assert_eq!(blend(a, b, 0.5), Color::Rgb(100, 50, 25));
        assert_eq!(blend(a, b, 2.0), b);
        assert_eq!(blend(Color::Red, Color::Blue, 0.4), Color::Red);
        assert_eq!(blend(Color::Red, Color::Blue, 0.6), Color::Blue);
    }

    #[test]
    fn level_resolution_respects_env_terminal_and_saved_choice() {
        assert_eq!(
            Level::resolve_with("full", Some("reduced"), None, false),
            Level::Reduced
        );
        assert_eq!(
            Level::resolve_with("full", None, Some("linux"), false),
            Level::Off
        );
        assert_eq!(Level::resolve_with("full", None, None, true), Level::Off);
        assert_eq!(
            Level::resolve_with("reduced", Some("full"), Some("dumb"), false),
            Level::Full
        );
        for level in Level::ALL {
            assert_eq!(Level::from_name(level.saved_name()), level);
        }
    }

    #[test]
    fn glides_ease_toward_new_targets_and_skip_when_off() {
        let mut m = Motion::new(Level::Full);
        m.pin(Some(1000.0));
        assert_eq!(m.glide("ctx", 0.2), 0.2, "first value is not animated");
        assert_eq!(
            m.glide("ctx", 0.8),
            0.2,
            "a change starts from the shown value"
        );
        assert!(m.active());
        m.pin(Some(1000.0 + GLIDE_MS / 2.0));
        let mid = m.glide("ctx", 0.8);
        assert!(mid > 0.5 && mid < 0.8, "{mid}");
        m.pin(Some(1000.0 + GLIDE_MS + 1.0));
        assert_eq!(m.glide("ctx", 0.8), 0.8);
        let mut off = Motion::new(Level::Off);
        off.glide("ctx", 0.1);
        assert_eq!(off.glide("ctx", 0.9), 0.9);
        assert!(!off.active());
    }

    #[test]
    fn page_and_modal_transitions_run_once_and_finish() {
        let mut m = Motion::new(Level::Full);
        m.splash_until = None;
        m.pin(Some(0.0));
        m.page(1);
        assert!(m.page_since.is_none(), "the first page does not animate");
        m.page(2);
        assert_eq!(m.progress(m.page_since, PAGE_MS), Some(0.0));
        m.pin(Some(PAGE_MS + 1.0));
        assert_eq!(m.progress(m.page_since, PAGE_MS), None);
        assert_eq!(m.modal(Some("menu".into())), Some(0.0));
        m.pin(Some(PAGE_MS + 50.0));
        assert!(m.modal(Some("menu".into())).is_some_and(|t| t > 0.2));
        m.modal_retained = Some(Retained {
            rect: Rect::new(0, 0, 1, 1),
            cells: vec![Default::default()],
        });
        m.pin(Some(1000.0));
        assert_eq!(m.modal(None), None);
        assert!(m.closing().is_some(), "closing dialogs fade out");
        m.pin(Some(1000.0 + MODAL_OUT_MS + 1.0));
        assert!(m.closing().is_none());
        assert!(!m.active());
    }

    #[test]
    fn toasts_collapse_replace_and_expire() {
        let mut m = Motion::new(Level::Reduced);
        m.pin(Some(0.0));
        m.toast("Saved", Tone::Good);
        m.toast("Saved", Tone::Good);
        assert_eq!(m.toasts.len(), 1);
        assert_eq!(m.toasts[0].count, 2);
        for text in ["a", "b", "c", "d"] {
            m.toast(text, Tone::Info);
        }
        assert_eq!(m.toasts.len(), 1, "a new notice replaces the old one");
        assert_eq!(m.toasts[0].text, "d");
        m.pin(Some(TOAST_MS + 1.0));
        assert!(m.toasts.is_empty());
        let mut off = Motion::new(Level::Off);
        off.toast("Saved", Tone::Good);
        assert!(off.toasts.is_empty(), "no toasts when motion is off");
    }

    #[test]
    fn streaming_replies_catch_up_and_history_is_immediate() {
        let mut m = Motion::new(Level::Full);
        m.pin(Some(0.0));
        assert_eq!(m.reveal(0, 500, false), 500, "history shows at once");
        assert_eq!(
            m.reveal(1, 400, true),
            0,
            "a new streaming reply starts hidden"
        );
        m.pin(Some(100.0));
        let shown = m.reveal(1, 400, true);
        assert!((120..400).contains(&shown), "{shown}");
        m.pin(Some(400.0));
        assert_eq!(m.reveal(1, 400, true), 400, "never far behind");
    }

    #[test]
    fn tool_status_changes_flash_once() {
        let mut m = Motion::new(Level::Full);
        m.pin(Some(0.0));
        assert_eq!(m.flash("t1", "pending"), None, "first sight does not flash");
        m.pin(Some(10.0));
        assert_eq!(m.flash("t1", "completed"), Some(0.0));
        m.pin(Some(10.0 + FLASH_MS));
        assert_eq!(m.flash("t1", "completed"), None);
    }

    #[test]
    fn splash_never_outlives_its_budget_and_can_be_dismissed() {
        let mut m = Motion::new(Level::Full);
        m.pin(Some(10.0));
        assert!(m.splash().is_some());
        m.dismiss_splash();
        assert!(m.splash().is_none());
        let mut m = Motion::new(Level::Full);
        m.pin(Some(SPLASH_MS));
        assert!(m.splash().is_none());
        assert!(Motion::new(Level::Reduced).splash_until.is_none());
    }
}
