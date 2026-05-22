//! 搜索工具结果渲染：Grep（content/count/files）、WebSearch、WebFetch

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::constants::NORMAL_RESULT_MAX_LINES;
use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;

// ── Grep 渲染 ──────────────────────────────────────────────────────────

/// Grep 结果：结构化搜索结果
///
/// 实际 Grep 工具输出格式（content 模式）：
/// ```text
/// 找到 5 个匹配:
///
/// src/command/chat/ui/chat.rs:123:let theme = Theme::load();
/// src/command/chat/ui/chat.rs:456:fn draw_help(t: &Theme) {
/// ```
///
/// files_with_matches 模式：
/// ```text
/// 找到 3 个匹配文件:
///
/// src/command/chat/ui/chat.rs
/// src/markdown/render.rs
/// ```
///
/// count 模式：
/// ```text
/// 共 10 处匹配:
///
/// src/command/chat/ui/chat.rs:5
/// src/markdown/render.rs:3
/// ```
pub(crate) fn render_grep_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let mut raw_lines: Vec<&str> = content.lines().collect();

    // 跳过第一行统计信息（如 "找到 5 个匹配:" 或 "共 10 处匹配:"）
    let header = raw_lines.first();
    if header.is_some_and(|h| h.contains("找到") || h.contains("共")) {
        raw_lines.remove(0);
    }

    // 过滤空行
    let all_lines: Vec<&str> = raw_lines.into_iter().filter(|l| !l.is_empty()).collect();

    if all_lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "    (无匹配)",
            Style::default().fg(theme.text_dim),
        )));
        return;
    }

    // 检测格式：尝试按 path:line:content 解析
    let is_content_format = all_lines
        .first()
        .is_some_and(|l| parse_grep_content_line(l).is_some());

    let is_count_format = all_lines.first().is_some_and(|l| {
        l.rfind(':')
            .is_some_and(|idx| l[idx + 1..].parse::<usize>().is_ok())
    });

    if is_content_format {
        render_grep_content_format(&all_lines, content_w, lines, theme);
    } else if is_count_format {
        render_grep_count_format(&all_lines, lines, theme);
    } else {
        // files_with_matches 格式 — 文件列表
        render_grep_files_format(&all_lines, lines, theme);
    }
}

/// 解析 path:lineno:content 格式行（匹配行，`:` 分隔）
/// 返回 (file_path, line_number, content)
fn parse_grep_content_line(line: &str) -> Option<(&str, usize, &str)> {
    let mut search_from = 0;
    while let Some(colon_idx) = line[search_from..].find(':') {
        let abs_idx = search_from + colon_idx;
        let after_colon = &line[abs_idx + 1..];
        if let Some(second_colon) = after_colon.find(':') {
            let line_num_str = &after_colon[..second_colon];
            if let Ok(line_num) = line_num_str.parse::<usize>() {
                let file_path = &line[..abs_idx];
                let content = &after_colon[second_colon + 1..];
                return Some((file_path, line_num, content));
            }
        }
        search_from = abs_idx + 1;
    }
    None
}

/// 解析上下文行格式（path-lineno:content，`-` 分隔路径和行号）
/// 返回 (file_path, line_number, content)
fn parse_grep_context_line(line: &str) -> Option<(&str, usize, &str)> {
    let mut search_from = 0;
    while let Some(dash_idx) = line[search_from..].find('-') {
        let abs_idx = search_from + dash_idx;
        let after_dash = &line[abs_idx + 1..];
        if let Some(colon_idx) = after_dash.find(':') {
            let line_num_str = &after_dash[..colon_idx];
            if let Ok(line_num) = line_num_str.parse::<usize>() {
                let file_path = &line[..abs_idx];
                let content = &after_dash[colon_idx + 1..];
                return Some((file_path, line_num, content));
            }
        }
        search_from = abs_idx + 1;
    }
    None
}

/// 渲染 content 模式的 Grep 结果
pub(crate) fn render_grep_content_format(
    all_lines: &[&str],
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let max_display = NORMAL_RESULT_MAX_LINES.min(50);
    let mut current_file: Option<&str> = None;
    let mut match_count = 0usize;
    let mut file_count = 0usize;
    let mut displayed = 0usize;

    for line in all_lines {
        if displayed >= max_display {
            break;
        }

        // 尝试匹配行格式: path:lineno:content
        if let Some((file_path, line_num, content)) = parse_grep_content_line(line) {
            if current_file != Some(file_path) {
                current_file = Some(file_path);
                file_count += 1;
                displayed += 1;
                lines.push(render_file_path_line(file_path, theme));
            }

            match_count += 1;
            displayed += 1;

            let wrapped = wrap_text(content, content_w.saturating_sub(14));
            for (i, w) in wrapped.iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled(
                            format!("{:>4} │ ", line_num),
                            Style::default().fg(theme.text_dim),
                        ),
                        Span::styled(
                            w.to_string(),
                            Style::default()
                                .fg(theme.config_title)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled("     │ ", Style::default().fg(theme.text_dim)),
                        Span::styled(
                            w.to_string(),
                            Style::default()
                                .fg(theme.config_title)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
            }
        }
        // 尝试上下文行格式: path-lineno:content
        else if let Some((file_path, line_num, content)) = parse_grep_context_line(line) {
            if current_file != Some(file_path) {
                current_file = Some(file_path);
                file_count += 1;
                displayed += 1;
                lines.push(render_file_path_line(file_path, theme));
            }

            displayed += 1;

            // 上下文行用 text_dim
            let wrapped = wrap_text(content, content_w.saturating_sub(14));
            for (i, w) in wrapped.iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled(
                            format!("{:>4} │ ", line_num),
                            Style::default().fg(theme.text_dim),
                        ),
                        Span::styled(w.to_string(), Style::default().fg(theme.text_dim)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled("     │ ", Style::default().fg(theme.text_dim)),
                        Span::styled(w.to_string(), Style::default().fg(theme.text_dim)),
                    ]));
                }
            }
        }
    }

    push_grep_summary(match_count, file_count, lines, theme);

    if all_lines.len() > max_display {
        lines.push(Line::from(Span::styled(
            "    ... (结果已截断)".to_string(),
            Style::default().fg(theme.text_dim),
        )));
    }
}

/// 渲染 count 模式的 Grep 结果
fn render_grep_count_format(all_lines: &[&str], lines: &mut Vec<Line<'static>>, theme: &Theme) {
    let mut total_count = 0usize;
    let mut file_count = 0usize;

    for line in all_lines {
        if let Some(last_colon) = line.rfind(':') {
            let file_path = &line[..last_colon];
            let count_str = &line[last_colon + 1..];
            if let Ok(count) = count_str.parse::<usize>() {
                total_count += count;
                file_count += 1;
                lines.push(Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(
                        file_path.to_string(),
                        Style::default().fg(theme.config_title),
                    ),
                    Span::styled(" — ".to_string(), Style::default().fg(theme.text_dim)),
                    Span::styled(
                        format!("{} 处", count),
                        Style::default().fg(theme.text_normal),
                    ),
                ]));
            }
        }
    }

    push_grep_summary(total_count, file_count, lines, theme);
}

/// 渲染 files_with_matches 模式的 Grep 结果
fn render_grep_files_format(all_lines: &[&str], lines: &mut Vec<Line<'static>>, theme: &Theme) {
    for line in all_lines {
        lines.push(render_file_path_line(line, theme));
    }

    if all_lines.len() > 1 {
        lines.push(Line::from(Span::styled(
            format!("    (共 {} 个文件)", all_lines.len()),
            Style::default().fg(theme.text_dim),
        )));
    }
}

/// 渲染文件路径行（带图标）
fn render_file_path_line(file_path: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(
            file_path.to_string(),
            Style::default()
                .fg(theme.config_title)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

/// 添加 Grep 统计行
fn push_grep_summary(
    match_count: usize,
    file_count: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    if match_count == 0 && file_count == 0 {
        return;
    }
    let summary = if file_count > 1 {
        format!("    (共 {} 处匹配，{} 个文件)", match_count, file_count)
    } else {
        format!("    (共 {} 处匹配)", match_count)
    };
    lines.push(Line::from(Span::styled(
        summary,
        Style::default().fg(theme.text_dim),
    )));
}

// ── WebSearch 渲染 ─────────────────────────────────────────────────────

/// WebSearch 结果：结构化搜索结果
///
/// 输入格式（由 format_search_results 生成）：
/// ```text
/// 搜索: query
///
/// 1. Title
///    URL
///    highlight text
///
/// 2. Title
///    URL
/// ```
pub(crate) fn render_web_search_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let mut result_count = 0usize;
    let mut iter = content.lines().peekable();

    // 跳过 "搜索: query" 首行
    if iter
        .peek()
        .is_some_and(|l| l.starts_with("搜索:") || l.starts_with("搜索："))
    {
        iter.next();
    }

    while let Some(line) = iter.next() {
        if line.is_empty() {
            continue;
        }

        // 检测序号行："1. Title"
        if let Some(rest) = line
            .trim_start()
            .strip_prefix(|c: char| c.is_ascii_digit())
            .and_then(|r| r.strip_prefix(". "))
        {
            result_count += 1;
            // 标题行
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("{}. ", result_count),
                    Style::default()
                        .fg(theme.config_title)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    rest.to_string(),
                    Style::default()
                        .fg(theme.text_normal)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            // 后续行：URL + 摘要
            let mut sub_lines = 0usize;
            while let Some(next) = iter.peek() {
                if next.is_empty()
                    || next
                        .trim_start()
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit() && next.contains(". "))
                {
                    break;
                }
                let sub_line = iter.next().unwrap_or("");
                sub_lines += 1;
                if sub_lines == 1 {
                    // URL 行
                    lines.push(Line::from(Span::styled(
                        format!("       {}", sub_line),
                        Style::default().fg(theme.text_dim),
                    )));
                } else {
                    // 摘要行
                    for wrapped in wrap_text(sub_line, content_w.saturating_sub(7)) {
                        lines.push(Line::from(Span::styled(
                            format!("       {}", wrapped),
                            Style::default().fg(theme.text_dim),
                        )));
                    }
                }
            }
            // 结果之间加空行
            lines.push(Line::from(""));
            continue;
        }

        // 非序号行，普通折行
        for wrapped in wrap_text(line, content_w.saturating_sub(4)) {
            lines.push(Line::from(Span::styled(
                format!("    {}", wrapped),
                Style::default().fg(theme.text_dim),
            )));
        }
    }

    // 统计行
    if result_count > 1 {
        lines.push(Line::from(Span::styled(
            format!("    (共 {} 个结果)", result_count),
            Style::default().fg(theme.text_dim),
        )));
    }
}

// ── WebFetch 渲染 ──────────────────────────────────────────────────────

/// WebFetch 结果：内容预览
///
/// 自动检测是否为 Markdown 内容：
/// - 包含 # 标题、- 列表等标记时用 markdown_to_lines 渲染
/// - 否则纯文本折行显示
pub(crate) fn render_web_fetch_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    // 检测是否为 Markdown 内容
    let has_markdown = content.lines().take(20).any(|l| {
        l.starts_with("# ")
            || l.starts_with("## ")
            || l.starts_with("- ")
            || l.starts_with("* ")
            || l.starts_with("> ")
            || l.starts_with("```")
            || l.starts_with("| ")
            || l.starts_with("1. ")
    });

    if has_markdown {
        // 用 IR 渲染器渲染 Markdown
        use crate::markdown::parser::markdown_to_lines;
        let md_lines = markdown_to_lines(content, content_w, theme);
        for md_line in md_lines {
            // 添加 4 空格缩进
            let mut spans = vec![Span::styled("    ", Style::default())];
            spans.extend(md_line.spans);
            lines.push(Line::from(spans));
        }
    } else {
        // 纯文本折行
        let all_lines: Vec<&str> = content.lines().take(NORMAL_RESULT_MAX_LINES).collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w.saturating_sub(4)) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_normal),
                )));
            }
        }
        let total = content.lines().count();
        if total > NORMAL_RESULT_MAX_LINES {
            lines.push(Line::from(Span::styled(
                format!(
                    "    ... (共 {} 行，显示前 {} 行)",
                    total, NORMAL_RESULT_MAX_LINES
                ),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}
