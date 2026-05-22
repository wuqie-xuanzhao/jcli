//! 确认/交互区域渲染：工具确认框、Ask 问答、权限确认、Plan 审批

mod ask_questions;
mod permission_confirm;
mod tool_confirm_content;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::app::ChatApp;

pub(crate) use ask_questions::render_ask_questions;
pub(crate) use permission_confirm::{
    render_agent_perm_confirm_area, render_plan_approval_confirm_area,
};
pub(crate) use tool_confirm_content::render_tool_confirm_content;

/// 渲染工具确认/Ask 交互区域
pub(crate) fn render_tool_confirm_area(
    app: &ChatApp,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let t = &app.ui.theme;
    let confirm_bg = t.tool_confirm_bg;
    let border_color = t.tool_confirm_border;
    let content_w = bubble_max_width.saturating_sub(6); // 左右各 3 的 padding
    let is_ask = app.ui.tool_ask_mode;

    // 空行
    lines.push(Line::from(""));

    // 标题行
    let title = if is_ask {
        "  🪐 あの、すみません… (〃´∀｀)ゞ"
    } else {
        "  🔧 工具调用确认"
    };
    lines.push(Line::from(Span::styled(
        title,
        Style::default()
            .fg(t.tool_confirm_title)
            .add_modifier(Modifier::BOLD),
    )));

    // 顶边框
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(confirm_bg),
    )));

    if is_ask {
        render_ask_questions(app, bubble_max_width, content_w, lines);
    } else if let Some(tc) = app
        .tool_executor
        .active_tool_calls
        .get(app.tool_executor.pending_tool_idx)
    {
        render_tool_confirm_content(app, tc, bubble_max_width, content_w, lines);
    }

    // 底边框
    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(confirm_bg),
    )));
}
