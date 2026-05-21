//! Text width utilities for Markdown rendering

const TAB_REPLACEMENT: &str = "    ";

/// Calculate the display width of a string (accounting for CJK wide chars, tabs, etc.)
pub fn display_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// Calculate the display width of a single character
pub fn char_width(c: char) -> usize {
    if c == '\t' {
        return TAB_REPLACEMENT.len();
    }
    if c.is_control() {
        return 0;
    }
    use unicode_width::UnicodeWidthChar;
    UnicodeWidthChar::width(c).unwrap_or(0)
}
