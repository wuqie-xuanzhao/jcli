//! 权限确认区域渲染：子 Agent 权限确认、Teammate Plan 审批

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::app::ChatApp;
use crate::command::chat::render::cache::PLAN_DISPLAY_MAX_LINES;
use crate::command::chat::render::cache::bubble::bordered_line;
use crate::util::text::wrap_text;

/// 渲染权限确认区域（子 Agent / Teammate 通用）
pub(crate) fn render_agent_perm_confirm_area(
    app: &ChatApp,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let t = &app.ui.theme;
    let confirm_bg = t.tool_confirm_bg;
    let border_color = t.tool_confirm_border;
    let content_w = bubble_max_width.saturating_sub(6);

    let req = match app.ui.pending_agent_perm.as_ref() {
        Some(r) => r,
        None => return,
    };

    // 顶边框（与 ask 弹窗对齐：带背景色）
    lines.push(Line::from(Span::styled(
        format!("  ╭{}╮", "─".repeat(bubble_max_width.saturating_sub(4))),
        Style::default().fg(border_color).bg(confirm_bg),
    )));

    // 标题行（支持折行）
    let title = req.title();
    let title_style = Style::default()
        .fg(t.tool_confirm_title)
        .add_modifier(Modifier::BOLD)
        .bg(confirm_bg);
    let title_wrapped = wrap_text(&title, content_w);
    for line_text in title_wrapped {
        lines.push(bordered_line(
            vec![Span::styled(line_text, title_style)],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

    // 工具名行
    lines.push(bordered_line(
        vec![Span::styled(
            format!(" 工具: {}", req.tool_name),
            Style::default()
                .fg(t.tool_confirm_name)
                .add_modifier(Modifier::BOLD)
                .bg(confirm_bg),
        )],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // 确认消息（折行显示）
    for wrapped in wrap_text(&req.confirm_msg, content_w) {
        lines.push(bordered_line(
            vec![Span::styled(
                format!(" {}", wrapped),
                Style::default().fg(t.tool_confirm_text).bg(confirm_bg),
            )],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

    // 空行间隔
    lines.push(bordered_line(
        vec![Span::styled(" ", Style::default().bg(confirm_bg))],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // Y/N 提示行
    lines.push(bordered_line(
        vec![Span::styled(
            " [Y/Enter] 允许   [N/Esc] 拒绝",
            Style::default()
                .fg(t.text_dim)
                .add_modifier(Modifier::BOLD)
                .bg(confirm_bg),
        )],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // 底边框（与 ask 弹窗对齐：带背景色）
    lines.push(Line::from(Span::styled(
        format!("  ╰{}╯", "─".repeat(bubble_max_width.saturating_sub(4))),
        Style::default().fg(border_color).bg(confirm_bg),
    )));
}

/// 渲染 Teammate Plan 审批确认区域
pub(crate) fn render_plan_approval_confirm_area(
    app: &ChatApp,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let t = &app.ui.theme;
    let confirm_bg = t.tool_confirm_bg;
    let border_color = t.tool_confirm_border;
    let content_w = bubble_max_width.saturating_sub(6);

    let req = match app.ui.pending_plan_approval.as_ref() {
        Some(r) => r,
        None => return,
    };

    // 顶边框（与 ask 弹窗对齐：带背景色）
    lines.push(Line::from(Span::styled(
        format!("  ╭{}╮", "─".repeat(bubble_max_width.saturating_sub(4))),
        Style::default().fg(border_color).bg(confirm_bg),
    )));

    // 标题行（支持折行）
    let title = format!(" Plan 审批请求 [{}] ", req.agent_name);
    let title_style = Style::default()
        .fg(t.tool_confirm_title)
        .add_modifier(Modifier::BOLD)
        .bg(confirm_bg);
    let title_wrapped = wrap_text(&title, content_w);
    for line_text in title_wrapped {
        lines.push(bordered_line(
            vec![Span::styled(line_text, title_style)],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

    // Plan 名称行（支持折行）
    let plan_name_text = format!(" Plan: {}", req.plan_name);
    let plan_name_style = Style::default()
        .fg(t.tool_confirm_name)
        .add_modifier(Modifier::BOLD)
        .bg(confirm_bg);
    for wrapped in wrap_text(&plan_name_text, content_w) {
        lines.push(bordered_line(
            vec![Span::styled(wrapped, plan_name_style)],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

    // Plan 内容（折行显示，最多 PLAN_DISPLAY_MAX_LINES 行）
    let plan_lines: Vec<&str> = req
        .plan_content
        .lines()
        .take(PLAN_DISPLAY_MAX_LINES)
        .collect();
    for line in &plan_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(bordered_line(
                vec![Span::styled(
                    format!(" {}", wrapped),
                    Style::default().fg(t.tool_confirm_text).bg(confirm_bg),
                )],
                bubble_max_width,
                border_color,
                confirm_bg,
            ));
        }
    }
    if req.plan_content.lines().count() > PLAN_DISPLAY_MAX_LINES {
        lines.push(bordered_line(
            vec![Span::styled(
                " ... (内容已截断)".to_string(),
                Style::default().fg(t.text_dim).bg(confirm_bg),
            )],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

    // 空行间隔
    lines.push(bordered_line(
        vec![Span::styled(" ", Style::default().bg(confirm_bg))],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // Y/N 提示行（支持折行）
    let hint_text = " [Y/Enter] 批准   [C] 批准并清空   [N/Esc] 拒绝";
    let hint_style = Style::default()
        .fg(t.text_dim)
        .add_modifier(Modifier::BOLD)
        .bg(confirm_bg);
    for wrapped in wrap_text(hint_text, content_w) {
        lines.push(bordered_line(
            vec![Span::styled(wrapped, hint_style)],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

    // 底边框（与 ask 弹窗对齐：带背景色）
    lines.push(Line::from(Span::styled(
        format!("  ╰{}╯", "─".repeat(bubble_max_width.saturating_sub(4))),
        Style::default().fg(border_color).bg(confirm_bg),
    )));
}
