//! 流式消息渲染构建
//!
//! 包含：
//! - `rebuild_streaming_only` - 仅重建流式内容（历史缓存有效时使用）
//! - `render_streaming_bubble` - 流式气泡渲染核心逻辑（共享辅助函数）

use crate::command::chat::app::{ChatApp, ChatMode, MsgLinesCache};
use crate::command::chat::render::theme::Theme;
use crate::command::chat::storage::config::ThinkingStyle;
use crate::markdown::markdown_to_lines;
use crate::util::safe_lock;
use crate::util::text::wrap_text;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use std::sync::Arc;

use crate::command::chat::render::cache::ASSISTANT_BUBBLE_LEFT_MARGIN;
use crate::command::chat::render::cache::animation::{
    comet_gradient_line, current_tick, thinking_pulse_color,
};
use crate::command::chat::render::cache::bubble::{
    wrap_md_line_in_bubble, wrap_md_line_in_bubble_with_margin,
};
use crate::command::chat::render::cache::confirm_render::{
    render_agent_perm_confirm_area, render_plan_approval_confirm_area, render_tool_confirm_area,
};
use crate::command::chat::render::cache::find_stable_boundary;

/// 仅重建流式内容（历史缓存有效时使用）
/// 避免遍历历史消息，直接复用旧缓存
/// 返回 (streaming_lines, stable_lines, stable_offset)
pub fn rebuild_streaming_only(
    app: &ChatApp,
    _inner_width: usize,
    bubble_max_width: usize,
    old_cache: Option<&MsgLinesCache>,
) -> (Vec<Line<'static>>, Arc<Vec<Line<'static>>>, usize) {
    let t = &app.ui.theme;

    // 获取流式内容
    let streaming_content_str = if app.state.is_loading {
        let streaming: String = safe_lock(
            &app.state.streaming_content,
            "rebuild_streaming::streaming_content",
        )
        .clone();
        if !streaming.is_empty() {
            Some(streaming)
        } else {
            None
        }
    } else {
        None
    };

    // 获取旧的 stable_lines
    let (stable_lines, old_stable_offset) = if let Some(old_c) = old_cache {
        if old_c.bubble_max_width == bubble_max_width {
            (
                (*old_c.streaming_stable_lines).clone(),
                old_c.streaming_stable_offset,
            )
        } else {
            (Vec::<Line<'static>>::new(), 0)
        }
    } else {
        (Vec::<Line<'static>>::new(), 0)
    };

    render_streaming_bubble(
        app,
        bubble_max_width,
        streaming_content_str.as_deref(),
        stable_lines,
        old_stable_offset,
        t,
    )
}

/// 流式气泡渲染核心逻辑
/// 被本模块的 `rebuild_streaming_only` 和 `history_build::build_message_lines_incremental` 共享
#[allow(clippy::too_many_arguments)]
pub fn render_streaming_bubble(
    app: &ChatApp,
    bubble_max_width: usize,
    streaming_content: Option<&str>,
    mut stable_lines: Vec<Line<'static>>,
    old_stable_offset: usize,
    t: &Theme,
) -> (Vec<Line<'static>>, Arc<Vec<Line<'static>>>, usize) {
    let mut streaming_lines: Vec<Line<'static>> = Vec::new();
    let has_streaming_msg = app.state.is_loading;
    let mut final_stable_offset = old_stable_offset;

    if has_streaming_msg {
        let streaming_text = streaming_content.unwrap_or("◍");
        let bubble_bg = if app.state.agent_config.flat_bubble {
            t.bg_primary
        } else {
            t.bubble_ai
        };
        let pad_left_w = 3usize;
        let pad_right_w = 3usize;
        let margin_str = " ".repeat(ASSISTANT_BUBBLE_LEFT_MARGIN);
        let md_content_w = bubble_max_width
            .saturating_sub(pad_left_w + pad_right_w + ASSISTANT_BUBBLE_LEFT_MARGIN);
        let inner_bubble_w = bubble_max_width.saturating_sub(ASSISTANT_BUBBLE_LEFT_MARGIN);

        // AI 标签
        streaming_lines.push(Line::from(""));
        streaming_lines.push(Line::from(Span::styled(
            format!("{}Sprite", margin_str),
            Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
        )));

        // 上边距
        streaming_lines.push(Line::from(vec![
            Span::styled(margin_str.clone(), Style::default()),
            Span::styled(" ".repeat(inner_bubble_w), Style::default().bg(bubble_bg)),
        ]));

        // 思考指示器
        if streaming_text == "◍" {
            let tick = current_tick();
            let thinking_style = app.state.agent_config.thinking_style;

            let indicator_line = if thinking_style == ThinkingStyle::Comet {
                comet_gradient_line(tick, t.welcome_palette, t.label_ai)
            } else {
                let pulse_color = thinking_pulse_color(t);
                let frame = thinking_style.frame(tick);
                Line::from(Span::styled(frame, Style::default().fg(pulse_color)))
            };
            let bubble_line = wrap_md_line_in_bubble_with_margin(
                indicator_line,
                bubble_bg,
                pad_left_w,
                pad_right_w,
                inner_bubble_w,
                &margin_str,
            );
            streaming_lines.push(bubble_line);

            // Reasoning 内容
            let reasoning_str = safe_lock(
                &app.state.streaming_reasoning_content,
                "render_streaming::streaming_reasoning",
            )
            .clone();
            if !reasoning_str.is_empty() {
                let bar_color = t.md_blockquote_bar;
                let quote_text_color = t.md_blockquote_text;
                let quote_bg = t.bg_primary;

                let thinking_label = Line::from(vec![
                    Span::styled(
                        "| ",
                        Style::default()
                            .fg(bar_color)
                            .bg(quote_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "Thinking...",
                        Style::default()
                            .fg(bar_color)
                            .bg(quote_bg)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]);
                let label_bubble = wrap_md_line_in_bubble_with_margin(
                    thinking_label,
                    bubble_bg,
                    pad_left_w,
                    pad_right_w,
                    inner_bubble_w,
                    &margin_str,
                );
                streaming_lines.push(label_bubble);

                let reason_content_w = md_content_w.saturating_sub(4);
                for wrapped_line in wrap_text(&reasoning_str, reason_content_w) {
                    let line = Line::from(vec![
                        Span::styled(
                            "| ",
                            Style::default()
                                .fg(bar_color)
                                .bg(quote_bg)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            wrapped_line,
                            Style::default().fg(quote_text_color).bg(quote_bg),
                        ),
                    ]);
                    let bubble_line = wrap_md_line_in_bubble_with_margin(
                        line,
                        bubble_bg,
                        pad_left_w,
                        pad_right_w,
                        inner_bubble_w,
                        &margin_str,
                    );
                    streaming_lines.push(bubble_line);
                }
            }

            // 下边距
            streaming_lines.push(Line::from(vec![
                Span::styled(margin_str.clone(), Style::default()),
                Span::styled(" ".repeat(inner_bubble_w), Style::default().bg(bubble_bg)),
            ]));
        } else {
            let content = streaming_text;
            let boundary = find_stable_boundary(content);

            if boundary > old_stable_offset {
                let new_stable_text = &content[old_stable_offset..boundary];
                let new_md_lines = markdown_to_lines(new_stable_text, md_content_w + 2, t);
                for md_line in new_md_lines {
                    let bubble_line = wrap_md_line_in_bubble(
                        md_line,
                        bubble_bg,
                        pad_left_w,
                        pad_right_w,
                        inner_bubble_w,
                    );
                    stable_lines.push(bubble_line);
                }
            }
            final_stable_offset = boundary;

            for sl in stable_lines.iter() {
                let mut line = sl.clone();
                line.spans
                    .insert(0, Span::styled(margin_str.clone(), Style::default()));
                streaming_lines.push(line);
            }

            let tail = &content[boundary..];
            if !tail.is_empty() {
                let tail_md_lines = markdown_to_lines(tail, md_content_w + 2, t);
                for md_line in tail_md_lines {
                    let bubble_line = wrap_md_line_in_bubble_with_margin(
                        md_line,
                        bubble_bg,
                        pad_left_w,
                        pad_right_w,
                        inner_bubble_w,
                        &margin_str,
                    );
                    streaming_lines.push(bubble_line);
                }
            }

            // 下边距
            streaming_lines.push(Line::from(vec![
                Span::styled(margin_str.clone(), Style::default()),
                Span::styled(" ".repeat(inner_bubble_w), Style::default().bg(bubble_bg)),
            ]));
        }
    } else {
        stable_lines = Vec::new();
        final_stable_offset = 0;
    }

    // 工具确认区
    if app.ui.mode == ChatMode::ToolConfirm {
        render_tool_confirm_area(app, bubble_max_width, &mut streaming_lines);
    }

    // Agent 权限确认区
    if app.ui.mode == ChatMode::AgentPermConfirm {
        render_agent_perm_confirm_area(app, bubble_max_width, &mut streaming_lines);
    }

    // Plan 审批确认区
    if app.ui.mode == ChatMode::PlanApprovalConfirm {
        render_plan_approval_confirm_area(app, bubble_max_width, &mut streaming_lines);
    }

    // 末尾留白
    streaming_lines.push(Line::from(""));

    (streaming_lines, Arc::new(stable_lines), final_stable_offset)
}
