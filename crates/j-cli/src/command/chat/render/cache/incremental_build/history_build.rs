//! 历史消息渲染缓存构建
//!
//! 包含：
//! - `build_message_lines_incremental` - 全量增量构建（遍历所有消息）
//! - `build_history_cache` - 历史消息缓存渲染（私有辅助函数）

use crate::command::chat::app::{ChatApp, ChatMode, MsgLinesCache, PerMsgCache};
use crate::command::chat::render::theme::Theme;
use crate::command::chat::storage::ChatMessage;
use crate::util::safe_lock;
use crate::util::text::wrap_text;
use ratatui::{
    style::Style,
    text::{Line, Span},
};
use std::sync::Arc;

use super::streaming_build::render_streaming_bubble;
use crate::command::chat::render::cache::RenderContext;
use crate::command::chat::render::cache::msg_render::{
    render_assistant_msg, render_thinking_block, render_user_msg,
};
use crate::command::chat::render::cache::tool_call_render::render_tool_call_request_msg;
use crate::command::chat::render::cache::tool_result_render::{
    ToolResultRenderParams, render_tool_result_msg,
};

/// 增量构建所有消息的渲染行（P0 + P1 + P2 优化版本）
/// - P0：按消息粒度缓存，历史消息内容未变时直接复用渲染行
/// - P1：流式消息增量段落渲染，只重新解析最后一个不完整段落
/// - P2：不再组装扁平 lines Vec，draw_messages 直接索引 per_msg_lines + streaming_lines
///   返回 (消息起始行号映射, 按消息缓存, 流式渲染行, 流式稳定行缓存, 流式稳定偏移)
///   注意：当历史缓存有效时，调用方应使用 rebuild_streaming_only() 代替此函数
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
pub fn build_message_lines_incremental(
    app: &ChatApp,
    inner_width: usize,
    bubble_max_width: usize,
    old_cache: Option<&MsgLinesCache>,
) -> (
    Vec<(usize, usize)>,
    Vec<PerMsgCache>,
    Vec<Line<'static>>,
    Arc<Vec<Line<'static>>>,
    usize,
) {
    let t = &app.ui.theme;
    let is_browse_mode = app.ui.mode == ChatMode::Browse;

    // ★ UI 渲染从 display_messages 读取（干净文本 + sender_name）
    let display_msgs = safe_lock(&app.display_messages, "render_cache::display_msgs");
    let expand = app.ui.expand_tools;

    // 构建历史消息缓存（增量复用旧缓存）
    let (msg_start_lines, per_msg_cache) = build_history_cache(&HistoryBuildParams {
        display_msgs: &display_msgs,
        old_cache,
        bubble_max_width,
        inner_width,
        t,
        expand,
        is_browse_mode,
        browse_msg_index: app.ui.browse_msg_index,
        flat_bubble: app.state.agent_config.flat_bubble,
    });

    // ===== 流式消息单独渲染进 streaming_lines =====
    // 获取流式内容（只 lock 一次，尽快释放锁）
    let streaming_content_str = if app.state.is_loading {
        let streaming: String = safe_lock(
            &app.state.streaming_content,
            "render_cache::streaming_content",
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

    // 获取旧的 stable_lines（Arc::clone O(1) 代替 Vec::clone O(n)）
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

    // 调用共享的流式渲染函数
    let (streaming_lines, stable_lines, final_stable_offset) = render_streaming_bubble(
        app,
        bubble_max_width,
        streaming_content_str.as_deref(),
        stable_lines,
        old_stable_offset,
        t,
    );

    (
        msg_start_lines,
        per_msg_cache,
        streaming_lines,
        stable_lines,
        final_stable_offset,
    )
}

/// 构建历史消息渲染缓存的参数。
struct HistoryBuildParams<'a> {
    display_msgs: &'a [ChatMessage],
    old_cache: Option<&'a MsgLinesCache>,
    bubble_max_width: usize,
    inner_width: usize,
    t: &'a Theme,
    expand: bool,
    is_browse_mode: bool,
    browse_msg_index: usize,
    flat_bubble: bool,
}

/// 构建历史消息渲染缓存（遍历所有消息）
#[allow(clippy::too_many_lines)]
/// 当历史缓存失效时调用，返回 (msg_start_lines, per_msg_cache)
fn build_history_cache(params: &HistoryBuildParams<'_>) -> (Vec<(usize, usize)>, Vec<PerMsgCache>) {
    use crate::command::chat::storage::DisplayType;

    let msg_count = params.display_msgs.len();
    let mut current_line_offset: usize = 0;
    let mut msg_start_lines: Vec<(usize, usize)> = Vec::with_capacity(msg_count);
    let mut per_msg_cache: Vec<PerMsgCache> = Vec::with_capacity(msg_count);

    // 判断旧缓存中的 per_msg_lines 是否可以复用（bubble_max_width 相同且 expand 一致）
    let can_reuse_per_msg = params
        .old_cache
        .map(|c| c.bubble_max_width == params.bubble_max_width && c.expand_tools == params.expand)
        .unwrap_or(false);

    for (idx, m) in params.display_msgs.iter().enumerate() {
        let is_selected = params.is_browse_mode && idx == params.browse_msg_index;

        // 记录消息起始行号
        msg_start_lines.push((idx, current_line_offset));

        // P0 优化：尝试直接按索引复用旧缓存
        if can_reuse_per_msg
            && let Some(old_c) = params.old_cache
            && let Some(old_per) = old_c.per_msg_lines.get(idx)
            && old_per.msg_index == idx
            && old_per.content_len == m.content.len()
            && old_per.is_selected == is_selected
        {
            // 直接复用旧缓存
            current_line_offset += old_per.lines.len();
            per_msg_cache.push(PerMsgCache {
                content_len: old_per.content_len,
                lines: old_per.lines.clone(),
                msg_index: idx,
                is_selected,
            });
            continue;
        }

        // 缓存未命中 → 重新渲染到临时 Vec
        let mut tmp_lines: Vec<Line<'static>> = Vec::new();
        match m.display_type() {
            DisplayType::User => {
                let mut ctx = RenderContext {
                    bubble_max_width: params.bubble_max_width,
                    lines: &mut tmp_lines,
                    theme: params.t,
                    expand: params.expand,
                    flat_bubble: params.flat_bubble,
                };
                render_user_msg(&m.content, is_selected, params.inner_width, &mut ctx);
            }
            DisplayType::AssistantText => {
                let mut ctx = RenderContext {
                    bubble_max_width: params.bubble_max_width,
                    lines: &mut tmp_lines,
                    theme: params.t,
                    expand: params.expand,
                    flat_bubble: params.flat_bubble,
                };
                if let Some(ref reasoning) = m.reasoning_content {
                    render_thinking_block(reasoning, &mut ctx);
                }
                render_assistant_msg(
                    m.sender_name.as_deref(),
                    m.recipient_name.as_deref(),
                    &m.content,
                    is_selected,
                    m.display_hint,
                    &mut ctx,
                );
            }
            DisplayType::ToolCallRequest => {
                let mut ctx = RenderContext {
                    bubble_max_width: params.bubble_max_width,
                    lines: &mut tmp_lines,
                    theme: params.t,
                    expand: params.expand,
                    flat_bubble: params.flat_bubble,
                };
                if let Some(ref reasoning) = m.reasoning_content {
                    render_thinking_block(reasoning, &mut ctx);
                }
                if !m.content.is_empty() {
                    render_assistant_msg(
                        m.sender_name.as_deref(),
                        m.recipient_name.as_deref(),
                        &m.content,
                        is_selected,
                        m.display_hint,
                        &mut ctx,
                    );
                }
                if let Some(ref tool_calls) = m.tool_calls {
                    render_tool_call_request_msg(m.sender_name.as_deref(), tool_calls, &mut ctx);
                }
            }
            DisplayType::ToolResult => {
                let tool_name = m
                    .tool_call_id
                    .as_ref()
                    .and_then(|tid| {
                        params.display_msgs[..idx].iter().rev().find_map(|prev| {
                            prev.tool_calls.as_ref().and_then(|tcs| {
                                tcs.iter()
                                    .find(|tc| tc.id == *tid)
                                    .map(|tc| tc.name.clone())
                            })
                        })
                    })
                    .unwrap_or_default();

                let tool_args = m.tool_call_id.as_ref().and_then(|tid| {
                    params.display_msgs[..idx].iter().rev().find_map(|prev| {
                        prev.tool_calls.as_ref().and_then(|tcs| {
                            tcs.iter()
                                .find(|tc| tc.id == *tid)
                                .map(|tc| tc.arguments.clone())
                        })
                    })
                });

                let label = if tool_name.is_empty() {
                    "工具结果".to_string()
                } else {
                    tool_name
                };

                render_tool_result_msg(
                    &ToolResultRenderParams {
                        sender_name: m.sender_name.as_deref(),
                        content: &m.content,
                        label: &label,
                        tool_args: tool_args.as_deref(),
                        bubble_max_width: params.bubble_max_width,
                        theme: params.t,
                        expand: params.expand,
                    },
                    &mut tmp_lines,
                );
            }
            DisplayType::System => {
                tmp_lines.push(Line::from(""));
                let wrapped = wrap_text(&m.content, params.inner_width.saturating_sub(8));
                for wl in wrapped {
                    tmp_lines.push(Line::from(Span::styled(
                        format!("    {}  {}", "sys", wl),
                        Style::default().fg(params.t.text_system),
                    )));
                }
            }
        }

        current_line_offset += tmp_lines.len();
        per_msg_cache.push(PerMsgCache {
            content_len: m.content.len(),
            lines: tmp_lines,
            msg_index: idx,
            is_selected,
        });
    }

    (msg_start_lines, per_msg_cache)
}
