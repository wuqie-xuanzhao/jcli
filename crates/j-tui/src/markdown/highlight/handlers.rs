use ratatui::{style::Style, text::Span};

use super::{ParseContext, flush_buf};

/// 处理双引号字符串（支持 `\` 转义）。
pub fn handle_double_quote(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '"' {
        return false;
    }
    flush_buf(ctx);
    let mut s = String::new();
    s.push(ch);
    ctx.chars.next();
    let mut escaped = false;
    while let Some(&c) = ctx.chars.peek() {
        s.push(c);
        ctx.chars.next();
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            break;
        }
    }
    ctx.spans.push(Span::styled(s, ctx.styles.str_style));
    true
}

/// 处理反引号字符串（不支持转义，遇到配对反引号结束）。
pub fn handle_backtick(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '`' {
        return false;
    }
    flush_buf(ctx);
    let mut s = String::new();
    s.push(ch);
    ctx.chars.next();
    while let Some(&c) = ctx.chars.peek() {
        s.push(c);
        ctx.chars.next();
        if c == '`' {
            break;
        }
    }
    ctx.spans.push(Span::styled(s, ctx.styles.str_style));
    true
}

/// 处理 Rust 生命周期参数 (`'a`, `'static` 等) vs 字符字面量 (`'x'`)。
pub fn handle_rust_lifetime(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '\'' || !matches!(ctx.lang, "rust" | "rs") {
        return false;
    }
    flush_buf(ctx);
    let mut s = String::new();
    s.push(ch);
    ctx.chars.next();
    let mut is_lifetime = false;
    while let Some(&c) = ctx.chars.peek() {
        if c.is_alphanumeric() || c == '_' {
            s.push(c);
            ctx.chars.next();
        } else if c == '\'' && s.len() == 2 {
            s.push(c);
            ctx.chars.next();
            break;
        } else {
            is_lifetime = true;
            break;
        }
    }
    if is_lifetime || (s.len() > 1 && !s.ends_with('\'')) {
        let lifetime_style = Style::default().fg(ctx.theme.code_lifetime);
        ctx.spans.push(Span::styled(s, lifetime_style));
    } else {
        ctx.spans.push(Span::styled(s, ctx.styles.str_style));
    }
    true
}

/// 处理非 Rust 语言的字符串（包含单引号）。
pub fn handle_single_quote(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '\'' || matches!(ctx.lang, "rust" | "rs") {
        return false;
    }
    flush_buf(ctx);
    let mut s = String::new();
    s.push(ch);
    ctx.chars.next();
    let mut escaped = false;
    while let Some(&c) = ctx.chars.peek() {
        s.push(c);
        ctx.chars.next();
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '\'' {
            break;
        }
    }
    ctx.spans.push(Span::styled(s, ctx.styles.str_style));
    true
}

/// 处理 Rust 属性 (`#[...]` 或 `#![...]`)。
pub fn handle_rust_attribute(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '#' || !matches!(ctx.lang, "rust" | "rs") {
        return false;
    }
    let mut lookahead = ctx.chars.clone();
    if let Some(next) = lookahead.next() {
        if next != '[' {
            return false;
        }
    } else {
        return false;
    }
    flush_buf(ctx);
    let mut attr = String::new();
    attr.push(ch);
    ctx.chars.next();
    let mut depth = 0;
    while let Some(&c) = ctx.chars.peek() {
        attr.push(c);
        ctx.chars.next();
        if c == '[' {
            depth += 1;
        } else if c == ']' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
    }
    let attr_style = Style::default().fg(ctx.theme.code_attribute);
    ctx.spans.push(Span::styled(attr, attr_style));
    true
}

/// 处理 Shell 变量 (`$VAR`, `${VAR}`, `$1` 等)。
pub fn handle_shell_variable(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '$'
        || !matches!(
            ctx.lang,
            "sh" | "bash" | "zsh" | "shell" | "dockerfile" | "docker"
        )
    {
        return false;
    }
    flush_buf(ctx);
    let var_style = Style::default().fg(ctx.theme.code_shell_var);
    let mut var = String::new();
    var.push(ch);
    ctx.chars.next();
    if let Some(&next_ch) = ctx.chars.peek() {
        if next_ch == '{' {
            var.push(next_ch);
            ctx.chars.next();
            while let Some(&c) = ctx.chars.peek() {
                var.push(c);
                ctx.chars.next();
                if c == '}' {
                    break;
                }
            }
        } else if next_ch == '(' {
            var.push(next_ch);
            ctx.chars.next();
            let mut depth = 1;
            while let Some(&c) = ctx.chars.peek() {
                var.push(c);
                ctx.chars.next();
                if c == '(' {
                    depth += 1;
                }
                if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
        } else if next_ch.is_alphanumeric()
            || next_ch == '_'
            || next_ch == '@'
            || next_ch == '#'
            || next_ch == '?'
            || next_ch == '!'
        {
            while let Some(&c) = ctx.chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    var.push(c);
                    ctx.chars.next();
                } else {
                    break;
                }
            }
        }
    }
    ctx.spans.push(Span::styled(var, var_style));
    true
}

/// 处理行内注释检测。
#[allow(clippy::too_many_arguments)]
pub fn handle_inline_comment(
    ch: char,
    ctx: &mut ParseContext<'_, '_>,
    comment_prefix: &str,
    comment_style: Style,
) -> bool {
    if ch != '/' && ch != '#' && ch != '-' {
        return false;
    }
    let rest: String = ctx.chars.clone().collect();
    if !rest.starts_with(comment_prefix) {
        return false;
    }
    flush_buf(ctx);
    while ctx.chars.peek().is_some() {
        ctx.chars.next();
    }
    ctx.spans.push(Span::styled(rest, comment_style));
    true
}
