//! 通用渲染：diff 着色、Agent 嵌套结果边框

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::command::chat::constants::AGENT_RESULT_MAX_LINES;
use crate::command::chat::render::cache::bubble::bordered_line;
use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;

/// 渲染包含 diff 块的工具结果内容
pub(crate) fn render_diff_content(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let mut in_diff = false;
    for line in content.lines() {
        if line.starts_with("```diff") {
            in_diff = true;
            continue;
        }
        if in_diff && line.starts_with("```") {
            in_diff = false;
            continue;
        }
        if in_diff {
            let color = if line.starts_with("- ")
                || line.starts_with('-') && !line.starts_with("---")
            {
                theme.diff_del
            } else if line.starts_with("+ ") || line.starts_with('+') && !line.starts_with("+++") {
                theme.diff_add
            } else if line.starts_with("@@ ") {
                theme.diff_header
            } else {
                theme.text_dim
            };
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(color),
                )));
            }
        } else {
            // diff 块外的文本正常渲染
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }
    }
}

/// 渲染 Agent 工具结果（嵌套缩进显示）
pub(crate) fn render_agent_result_nested(
    content: &str,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let all_lines: Vec<&str> = content.lines().collect();
    let total = all_lines.len();
    let max_display = AGENT_RESULT_MAX_LINES;
    let display_lines = &all_lines[..total.min(max_display)];

    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    // bordered_line: 左 "  │ " (4) + 右 " │" (2) = 6 开销
    let content_w = bubble_max_width.saturating_sub(6);

    // 顶边框
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    // 内容行
    for line in display_lines.iter() {
        for wrapped in wrap_text(line, content_w) {
            lines.push(bordered_line(
                vec![Span::styled(
                    wrapped,
                    Style::default().fg(theme.text_dim).bg(result_bg),
                )],
                bubble_max_width,
                border_color,
                result_bg,
            ));
        }
    }

    // 截断提示
    if total > max_display {
        lines.push(bordered_line(
            vec![Span::styled(
                format!("... (共 {} 行)", total),
                Style::default().fg(theme.text_dim).bg(result_bg),
            )],
            bubble_max_width,
            border_color,
            result_bg,
        ));
    }

    // 底边框
    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
}
