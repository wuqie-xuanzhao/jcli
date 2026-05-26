use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use super::{ParseContext, flush_buf};

/// 处理 YAML 文档分隔符 `---` 和 `...`
pub fn handle_yaml_document_marker(_ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") {
        return false;
    }
    let rest: String = ctx.chars.clone().collect();
    if rest.starts_with("---") || rest.starts_with("...") {
        flush_buf(ctx);
        let marker = if rest.starts_with("---") {
            "---"
        } else {
            "..."
        };
        for _ in 0..marker.len() {
            ctx.chars.next();
        }
        ctx.spans
            .push(Span::styled(marker.to_string(), ctx.styles.kw_style));
        return true;
    }
    false
}

/// 处理 YAML 类型标签 `!!str`, `!!int`, `!!bool` 等
pub fn handle_yaml_tag(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") || ch != '!' {
        return false;
    }
    let rest: String = ctx.chars.clone().collect();
    if !rest.starts_with("!!") {
        return false;
    }
    flush_buf(ctx);
    let mut tag = String::new();
    tag.push('!');
    ctx.chars.next();
    tag.push('!');
    ctx.chars.next();
    while let Some(&c) = ctx.chars.peek() {
        if c.is_alphanumeric() {
            tag.push(c);
            ctx.chars.next();
        } else {
            break;
        }
    }
    let tag_style = Style::default()
        .fg(ctx.theme.code_comment)
        .add_modifier(Modifier::ITALIC);
    ctx.spans.push(Span::styled(tag, tag_style));
    true
}

/// 处理 YAML 锚点 `&anchor`、别名 `*alias`、合并键 `<<`
pub fn handle_yaml_anchor(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") {
        return false;
    }
    // 处理 `<<` 合并键（后面可能有 `:`）
    let rest: String = ctx.chars.clone().collect();
    if rest.starts_with("<<") {
        flush_buf(ctx);
        ctx.chars.next();
        ctx.chars.next();
        let anchor_style = Style::default().fg(ctx.theme.code_attribute);
        ctx.spans.push(Span::styled("<<", anchor_style));
        return true;
    }
    // 处理 `&anchor` 或 `*alias`
    if ch != '&' && ch != '*' {
        return false;
    }
    flush_buf(ctx);
    let mut anchor = String::new();
    anchor.push(ch);
    ctx.chars.next();
    while let Some(&c) = ctx.chars.peek() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            anchor.push(c);
            ctx.chars.next();
        } else {
            break;
        }
    }
    let anchor_style = Style::default().fg(ctx.theme.code_attribute);
    ctx.spans.push(Span::styled(anchor, anchor_style));
    true
}

/// 处理 YAML 块标量指示符 `|` 或 `>` 及其修饰符
pub fn handle_yaml_block_scalar(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") || (ch != '|' && ch != '>') {
        return false;
    }
    flush_buf(ctx);
    let mut indicator = String::new();
    indicator.push(ch);
    ctx.chars.next();
    // 可选修饰符：`-`（strip）、`+`（keep）、数字（缩进）
    while let Some(&c) = ctx.chars.peek() {
        if c == '-' || c == '+' || c.is_ascii_digit() {
            indicator.push(c);
            ctx.chars.next();
        } else {
            break;
        }
    }
    ctx.spans
        .push(Span::styled(indicator, ctx.styles.macro_style));
    true
}

/// 处理 YAML 键名（冒号前的标识符）
///
/// YAML 键名格式：`key:` 或 `key :`（冒号后通常是空格或换行）
/// 检测逻辑：标识符后紧跟冒号，且冒号后是空格/换行/结束
pub fn handle_yaml_key(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") {
        return false;
    }
    // 只处理标识符起始字符
    if !ch.is_alphanumeric() && ch != '_' {
        return false;
    }
    // 检查后续是否形成键名模式
    let rest: String = ctx.chars.clone().collect();
    // 检查是否后面有冒号（键名结束）
    if let Some(colon_pos) = rest.find(':') {
        let before_colon = &rest[..colon_pos];
        // 确保冒号前只有合法的键名字符（字母、数字、下划线、连字符、点）
        if !before_colon
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return false;
        }
        // 检查冒号后是否是有效的值分隔（空格、换行、结束）
        let after_colon = &rest[colon_pos + 1..];
        let is_key = after_colon.is_empty()
            || after_colon.starts_with(' ')
            || after_colon.starts_with('\n')
            || after_colon.starts_with('\t');
        if !is_key {
            return false;
        }
        // 确认是键名：刷新 buf，读取键名部分
        flush_buf(ctx);
        let mut key = String::new();
        while let Some(&c) = ctx.chars.peek() {
            if c == ':' {
                ctx.chars.next();
                break;
            }
            key.push(c);
            ctx.chars.next();
        }
        // 键名用 type 风格，冒号用 default
        ctx.spans.push(Span::styled(key, ctx.styles.type_style));
        ctx.spans.push(Span::styled(":", ctx.styles.default_style));
        return true;
    }
    false
}

/// 处理 YAML 列表指示符 `-`
///
/// 只在行首（或仅有缩进空格后）识别为列表指示符。
/// 检测条件：buf 为空（意味着刚处理完行首空格），且后续是空格或内容。
pub fn handle_yaml_list_indicator(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") || ch != '-' {
        return false;
    }
    // 只在 buf 为空时识别（即行首或仅有缩进空格后）
    if !ctx.buf.is_empty() {
        return false;
    }
    let rest: String = ctx.chars.clone().collect();
    // 列表指示符后面必须是空格或内容（不能是 `---` 文档分隔符）
    if rest.starts_with("---") {
        return false;
    }
    // 检查 `-` 后面是否是空格（标准列表格式）或直接是内容
    let mut lookahead = ctx.chars.clone();
    lookahead.next(); // skip '-'
    if let Some(&after_dash) = lookahead.peek() {
        // `- ` 或 `-word` 都是有效列表项
        if after_dash == ' '
            || after_dash.is_alphanumeric()
            || after_dash == '"'
            || after_dash == '\''
        {
            flush_buf(ctx);
            ctx.chars.next(); // consume '-'
            ctx.spans.push(Span::styled("-", ctx.styles.kw_style));
            return true;
        }
    }
    false
}
