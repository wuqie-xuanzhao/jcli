use ratatui::{style::Style, text::Span};

use super::{SyntaxDicts, SyntaxStyles};

/// 将文本按照 word boundary 拆分并对关键字、数字、类型名、原始类型着色
#[allow(clippy::too_many_arguments)]
pub fn colorize_tokens(
    text: &str,
    dicts: &SyntaxDicts<'_>,
    styles: &SyntaxStyles,
    lang: &str,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current_word = String::new();
    let mut current_non_word = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() || ch == '_' {
            if !current_non_word.is_empty() {
                spans.push(Span::styled(current_non_word.clone(), styles.default_style));
                current_non_word.clear();
            }
            current_word.push(ch);
        } else {
            // Rust 宏调用高亮：word! 或 word!()
            if ch == '!' && matches!(lang, "rust" | "rs") && !current_word.is_empty() {
                let is_macro = chars
                    .peek()
                    .map(|&c| c == '(' || c == '{' || c == '[' || c.is_whitespace())
                    .unwrap_or(true);
                if is_macro {
                    spans.push(Span::styled(current_word.clone(), styles.macro_style));
                    current_word.clear();
                    spans.push(Span::styled("!".to_string(), styles.macro_style));
                    continue;
                }
            }
            if !current_word.is_empty() {
                let style = classify_word(&current_word, dicts, styles, lang);
                spans.push(Span::styled(current_word.clone(), style));
                current_word.clear();
            }
            current_non_word.push(ch);
        }
    }

    // 刷新剩余
    if !current_non_word.is_empty() {
        spans.push(Span::styled(current_non_word, styles.default_style));
    }
    if !current_word.is_empty() {
        let style = classify_word(&current_word, dicts, styles, lang);
        spans.push(Span::styled(current_word, style));
    }

    spans
}

/// 根据语言规则判断一个 word 应该使用哪种颜色样式
#[allow(clippy::too_many_arguments)]
pub fn classify_word(
    word: &str,
    dicts: &SyntaxDicts<'_>,
    styles: &SyntaxStyles,
    lang: &str,
) -> Style {
    if dicts.keywords.contains(&word) {
        styles.kw_style
    } else if dicts.primitive_types.contains(&word) {
        styles.primitive_style
    } else if word
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        styles.num_style
    } else if matches!(lang, "go" | "golang") {
        if dicts.go_type_names.contains(&word) {
            styles.type_style
        } else {
            styles.default_style
        }
    } else if word
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        styles.type_style
    } else {
        styles.default_style
    }
}
