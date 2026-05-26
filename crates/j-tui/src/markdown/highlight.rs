mod dicts;
mod handlers;
mod tokenize;
mod yaml;

use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use crate::editor_core::EditorTheme;

pub use tokenize::{classify_word, colorize_tokens};

/// 语法高亮样式集合
pub struct SyntaxStyles {
    pub default_style: Style,
    pub kw_style: Style,
    pub str_style: Style,
    pub num_style: Style,
    pub type_style: Style,
    pub primitive_style: Style,
    pub macro_style: Style,
}

/// 语法关键字字典
pub struct SyntaxDicts<'a> {
    pub keywords: &'a [&'a str],
    pub primitive_types: &'a [&'a str],
    pub go_type_names: &'a [&'a str],
}

/// 解析过程中的共享上下文，减少 handle_* 函数的参数数量。
struct ParseContext<'a, 'b> {
    chars: &'a mut std::iter::Peekable<std::str::Chars<'b>>,
    buf: &'a mut String,
    spans: &'a mut Vec<Span<'static>>,
    dicts: &'a SyntaxDicts<'a>,
    styles: &'a SyntaxStyles,
    lang: &'a str,
    theme: &'a EditorTheme,
}

/// 简单的代码语法高亮（无需外部依赖）
///
/// 根据语言类型对常见关键字、字符串、注释、数字进行着色。
#[allow(clippy::too_many_lines)]
pub fn highlight_code_line(line: &str, lang: &str, theme: &EditorTheme) -> Vec<Span<'static>> {
    let lang_lower = lang.to_lowercase();
    let lang_str = lang_lower.as_str();

    let keywords = dicts::keywords_for_lang(lang_str);
    let primitive_types = dicts::primitive_types_for_lang(lang_str);
    let go_type_names = dicts::go_type_names_for_lang(lang_str);
    let comment_prefix = dicts::comment_prefix_for_lang(lang_str);

    // ===== 代码高亮配色方案（基于主题）=====
    let styles = SyntaxStyles {
        default_style: Style::default().fg(theme.code_default),
        kw_style: Style::default().fg(theme.code_keyword),
        str_style: Style::default().fg(theme.code_string),
        num_style: Style::default().fg(theme.code_number),
        type_style: Style::default().fg(theme.code_type),
        primitive_style: Style::default().fg(theme.code_primitive),
        macro_style: Style::default().fg(theme.code_macro),
    };
    let comment_style = Style::default()
        .fg(theme.code_comment)
        .add_modifier(Modifier::ITALIC);

    let dicts = SyntaxDicts {
        keywords,
        primitive_types,
        go_type_names,
    };

    let trimmed = line.trim_start();

    // 注释行
    if trimmed.starts_with(comment_prefix) {
        return vec![Span::styled(line.to_string(), comment_style)];
    }

    // 逐词解析
    let mut spans = Vec::new();
    let mut chars = line.chars().peekable();
    let mut buf = String::new();

    let mut ctx = ParseContext {
        chars: &mut chars,
        buf: &mut buf,
        spans: &mut spans,
        dicts: &dicts,
        styles: &styles,
        lang: &lang_lower,
        theme,
    };

    while let Some(&ch) = ctx.chars.peek() {
        // --- YAML 专用高亮（优先级最高）---
        if yaml::handle_yaml_document_marker(ch, &mut ctx) {
            continue;
        }
        if yaml::handle_yaml_tag(ch, &mut ctx) {
            continue;
        }
        if yaml::handle_yaml_anchor(ch, &mut ctx) {
            continue;
        }
        if yaml::handle_yaml_block_scalar(ch, &mut ctx) {
            continue;
        }
        if yaml::handle_yaml_list_indicator(ch, &mut ctx) {
            continue;
        }
        if yaml::handle_yaml_key(ch, &mut ctx) {
            continue;
        }
        // --- 通用高亮 ---
        if handlers::handle_double_quote(ch, &mut ctx) {
            continue;
        }
        if handlers::handle_backtick(ch, &mut ctx) {
            continue;
        }
        if handlers::handle_rust_lifetime(ch, &mut ctx) {
            continue;
        }
        if handlers::handle_single_quote(ch, &mut ctx) {
            continue;
        }
        if handlers::handle_rust_attribute(ch, &mut ctx) {
            continue;
        }
        if handlers::handle_shell_variable(ch, &mut ctx) {
            continue;
        }
        if handlers::handle_inline_comment(ch, &mut ctx, comment_prefix, comment_style) {
            continue;
        }
        ctx.buf.push(ch);
        ctx.chars.next();
    }

    if !ctx.buf.is_empty() {
        ctx.spans
            .extend(colorize_tokens(ctx.buf, ctx.dicts, ctx.styles, ctx.lang));
    }

    if ctx.spans.is_empty() {
        ctx.spans
            .push(Span::styled(line.to_string(), ctx.styles.default_style));
    }

    spans
}

/// 刷新 buf 中累积的普通文本为着色 token。
fn flush_buf(ctx: &mut ParseContext<'_, '_>) {
    if !ctx.buf.is_empty() {
        ctx.spans
            .extend(colorize_tokens(ctx.buf, ctx.dicts, ctx.styles, ctx.lang));
        ctx.buf.clear();
    }
}
