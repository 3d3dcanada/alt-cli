use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Default)]
pub struct Editor {
    pub text: String,
    pub cursor: usize,
}

impl Editor {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.len();
        Self { text, cursor }
    }
    pub fn replace(&mut self, text: impl Into<String>) {
        *self = Self::new(text);
    }
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }
    pub fn insert(&mut self, text: &str, multiline: bool) {
        let clean: String = text
            .chars()
            .filter_map(|c| {
                if c == '\n' {
                    Some(if multiline { '\n' } else { ' ' })
                } else if c == '\t' {
                    Some(if multiline { '\t' } else { ' ' })
                } else if !c.is_control() {
                    Some(c)
                } else {
                    None
                }
            })
            .collect();
        if self.text.len() + clean.len() > 64 * 1024 {
            return;
        }
        self.text.insert_str(self.cursor, &clean);
        self.cursor += clean.len();
    }
    fn previous(&self) -> usize {
        self.text[..self.cursor]
            .grapheme_indices(true)
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
    fn next(&self) -> usize {
        self.text[self.cursor..]
            .graphemes(true)
            .next()
            .map(|g| self.cursor + g.len())
            .unwrap_or(self.text.len())
    }
    pub fn key(&mut self, key: KeyEvent, multiline: bool) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('a') if ctrl => self.cursor = 0,
            KeyCode::Char('e') if ctrl => self.cursor = self.text.len(),
            KeyCode::Char('u') if ctrl => self.clear(),
            KeyCode::Char(c) if !ctrl => self.insert(&c.to_string(), multiline),
            KeyCode::Left => self.cursor = self.previous(),
            KeyCode::Right => self.cursor = self.next(),
            KeyCode::Backspace => {
                let previous = self.previous();
                self.text.drain(previous..self.cursor);
                self.cursor = previous;
            }
            KeyCode::Delete => {
                let next = self.next();
                self.text.drain(self.cursor..next);
            }
            KeyCode::Home => {
                self.cursor = self.text[..self.cursor]
                    .rfind('\n')
                    .map(|n| n + 1)
                    .unwrap_or(0)
            }
            KeyCode::End => {
                self.cursor = self.text[self.cursor..]
                    .find('\n')
                    .map(|n| self.cursor + n)
                    .unwrap_or(self.text.len())
            }
            KeyCode::Enter if multiline => self.insert("\n", true),
            KeyCode::Up | KeyCode::Down if multiline => {
                let start = self.text[..self.cursor]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let column = self.text[start..self.cursor].graphemes(true).count();
                let range = if key.code == KeyCode::Up {
                    if start == 0 {
                        return;
                    }
                    let end = start - 1;
                    let begin = self.text[..end].rfind('\n').map(|i| i + 1).unwrap_or(0);
                    (begin, end)
                } else {
                    let Some(end) = self.text[self.cursor..].find('\n').map(|i| self.cursor + i)
                    else {
                        return;
                    };
                    let begin = end + 1;
                    let end = self.text[begin..]
                        .find('\n')
                        .map(|i| begin + i)
                        .unwrap_or(self.text.len());
                    (begin, end)
                };
                self.cursor = self.text[range.0..range.1]
                    .grapheme_indices(true)
                    .nth(column)
                    .map(|(i, _)| range.0 + i)
                    .unwrap_or(range.1);
            }
            _ => {}
        }
    }
    /// One-line horizontal viewport measured in terminal cells, not bytes.
    pub fn line_view(&self, width: usize) -> (String, u16) {
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let mut start = line_start;
        for (i, g) in self.text[line_start..self.cursor].grapheme_indices(true) {
            if self.text[start..self.cursor].width() < width.max(1) {
                break;
            }
            start = line_start + i + g.len();
        }
        let text = self.text[start..]
            .split('\n')
            .next()
            .unwrap_or("")
            .to_string();
        (
            text,
            self.text[start..self.cursor].width().min(u16::MAX as usize) as u16,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pasted_source_preserves_tabs_and_accepts_the_documented_editor_limit() {
        let source = "all:\n\tprintf 'hello\\n'\n";
        let mut file = Editor::default();
        file.insert(source, true);
        assert_eq!(
            file.text, source,
            "A pasted Makefile must retain its recipe tab"
        );
        let mut field = Editor::default();
        field.insert("one\ttwo\nthree", false);
        assert_eq!(field.text, "one two three");
        file.replace("x".repeat(64 * 1024 - 1));
        file.insert("y", true);
        assert_eq!(file.text.len(), 64 * 1024);
    }
    #[test]
    fn unicode_editing_and_paste_preserve_boundaries() {
        let mut e = Editor::new("A🙂e\u{301}中");
        e.key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), true);
        e.key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), true);
        assert_eq!(e.text, "A🙂中");
        e.insert("\nhello\x1b", true);
        assert_eq!(e.text, "A🙂\nhello中");
        e.key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE), true);
        assert_eq!(e.cursor, 6);
        e.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), true);
        assert_eq!(e.cursor, 0);
        assert!(e.line_view(4).1 < 4);
    }
}
