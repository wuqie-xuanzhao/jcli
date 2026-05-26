//! 消息列表绘制模块
//!
//! 绘制消息列表区域，支持缓存增量渲染、图片加载和滚动定位。

use super::render_image::render_image_pass;
use super::render_text::{TextPassParams, render_text_pass};
use crate::command::chat::app::{ChatApp, ChatMode, MsgLinesCache};
use crate::command::chat::render::cache::build_message_lines_incremental;
use crate::command::chat::ui::components;
use crate::util::safe_lock;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
};

/// 消息气泡宽度占内部可用宽度的百分比。
const BUBBLE_WIDTH_PERCENT: usize = 85;
/// 气泡渲染布局版本：边框样式变化时递增此值以强制缓存失效
const RENDER_VERSION: u32 = 2;

/// 绘制消息列表区域，支持缓存增量渲染、图片加载和滚动定位
#[allow(clippy::too_many_lines)]
pub fn draw_messages(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.ui.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_message))
        .style(Style::default().bg(t.bg_primary));

    // 空消息时显示欢迎界面（以 display_messages 为准，它是 UI 的数据源）
    // 快速检查是否为空，锁住后立即释放
    let is_empty = {
        let display = crate::util::safe_lock(&app.display_messages, "draw_messages::is_empty");
        display.is_empty()
    };
    if is_empty && !app.state.is_loading {
        let inner_width = area.width.saturating_sub(4);
        let welcome_lines = components::welcome_box(
            inner_width,
            t,
            app.ui.quote_idx,
            app.state.agent_config.welcome_quote,
        );
        let empty = Paragraph::new(welcome_lines).block(block);
        f.render_widget(empty, area);
        return;
    }

    // 内部可用宽度（减去边框和左右各1的 padding）
    let inner_width = area.width.saturating_sub(4) as usize;
    // 消息内容最大宽度为可用宽度的指定百分比
    let bubble_max_width = (inner_width * BUBBLE_WIDTH_PERCENT / 100).max(20);

    let (msg_count, last_msg_len) = {
        let display = crate::util::safe_lock(&app.display_messages, "draw_messages::msg_stats");
        (
            display.len(),
            display.last().map(|m| m.content.len()).unwrap_or(0),
        )
    };
    let streaming_len = if app.state.is_loading {
        safe_lock(
            &app.state.streaming_content,
            "draw_messages::streaming_content",
        )
        .len()
    } else {
        0
    };
    let current_browse_index = if app.ui.mode == ChatMode::Browse {
        Some(app.ui.browse_msg_index)
    } else {
        None
    };
    let current_tool_confirm_idx = if app.ui.mode == ChatMode::ToolConfirm {
        Some(app.tool_executor.pending_tool_idx)
    } else {
        None
    };
    // === 分离历史缓存和流式缓存的命中判断 ===
    // 历史缓存：消息数量、内容、气泡宽度变化时才需要重建
    // 流式缓存：每帧重新渲染，但复用 stable_lines 增量缓存
    let history_cache_valid = if let Some(ref cache) = app.ui.msg_lines_cache {
        cache.msg_count == msg_count
            && cache.bubble_max_width == bubble_max_width
            && cache.expand_tools == app.ui.expand_tools
            && cache.browse_index == current_browse_index
            && cache.render_version == RENDER_VERSION
            // 验证每条消息内容长度一致（避免遍历全部，只检查数量和最后一条）
            && cache.per_msg_lines.len() == msg_count
            && cache.last_msg_len == last_msg_len
    } else {
        false
    };

    // 流式内容需要更新的条件
    let streaming_needs_update = app.state.is_loading
        || app
            .ui
            .msg_lines_cache
            .as_ref()
            .map(|c| c.streaming_len != streaming_len)
            .unwrap_or(true)
        || app
            .ui
            .msg_lines_cache
            .as_ref()
            .map(|c| c.tool_confirm_idx != current_tool_confirm_idx)
            .unwrap_or(true);

    // 缓存完全命中：历史有效且流式无需更新
    let cache_hit = history_cache_valid && !streaming_needs_update;

    if !cache_hit {
        if history_cache_valid {
            // ★ P3 核心优化：历史缓存有效，只重建流式内容
            // 避免遍历 1500 条消息、避免 clone Vec<PerMsgCache>
            let old_cache = app.ui.msg_lines_cache.take();
            // SAFETY: history_cache_valid 为 true 时，msg_lines_cache 必然存在
            let old_cache = old_cache.expect("history_cache_valid 检查已通过，缓存应存在");
            let (new_streaming_lines, new_stable_lines, new_stable_offset) =
                crate::command::chat::render::cache::rebuild_streaming_only(
                    app,
                    inner_width,
                    bubble_max_width,
                    Some(&old_cache),
                );
            let new_streaming_len = new_streaming_lines.len();
            // history_line_count 不变，total_line_count = history + streaming
            let total_line_count = old_cache.history_line_count + new_streaming_len;
            app.ui.msg_lines_cache = Some(MsgLinesCache {
                msg_count: old_cache.msg_count,
                last_msg_len: old_cache.last_msg_len,
                streaming_len,
                bubble_max_width: old_cache.bubble_max_width,
                browse_index: old_cache.browse_index,
                tool_confirm_idx: current_tool_confirm_idx,
                total_line_count,
                history_line_count: old_cache.history_line_count,
                msg_start_lines: old_cache.msg_start_lines,
                per_msg_lines: old_cache.per_msg_lines,
                streaming_lines: new_streaming_lines,
                streaming_stable_lines: new_stable_lines,
                streaming_stable_offset: new_stable_offset,
                expand_tools: old_cache.expand_tools,
                render_version: old_cache.render_version,
            });
        } else {
            // 历史缓存也失效，完整重建
            let old_cache = app.ui.msg_lines_cache.take();
            let (
                new_msg_start_lines,
                new_per_msg,
                new_streaming_lines,
                new_stable_lines,
                new_stable_offset,
            ) = build_message_lines_incremental(
                app,
                inner_width,
                bubble_max_width,
                old_cache.as_ref(),
            );
            let total_line_count: usize = new_per_msg.iter().map(|p| p.lines.len()).sum::<usize>()
                + new_streaming_lines.len();
            let history_line_count: usize = new_per_msg.iter().map(|p| p.lines.len()).sum();
            app.ui.msg_lines_cache = Some(MsgLinesCache {
                msg_count,
                last_msg_len,
                streaming_len,
                bubble_max_width,
                browse_index: current_browse_index,
                tool_confirm_idx: current_tool_confirm_idx,
                total_line_count,
                history_line_count,
                msg_start_lines: new_msg_start_lines,
                per_msg_lines: new_per_msg,
                streaming_lines: new_streaming_lines,
                streaming_stable_lines: new_stable_lines,
                streaming_stable_offset: new_stable_offset,
                expand_tools: app.ui.expand_tools,
                render_version: RENDER_VERSION,
            });
        }
    }

    let cached = match app.ui.msg_lines_cache.as_ref() {
        Some(c) => c,
        None => return,
    };
    // 使用 usize 避免超过 65535 行时溢出
    let total_lines = cached.total_line_count;

    f.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    // 缓存 inner rect 供鼠标事件处理使用
    app.ui.msg_area_inner = Some(inner);
    let visible_height = inner.height as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);

    if app.ui.mode != ChatMode::Browse {
        if matches!(
            app.ui.mode,
            ChatMode::ToolConfirm | ChatMode::AgentPermConfirm | ChatMode::PlanApprovalConfirm
        ) {
            if app.ui.auto_scroll
                || app.ui.scroll_offset == usize::MAX
                || app.ui.scroll_offset > max_scroll
            {
                app.ui.scroll_offset = max_scroll;
            }
        } else if app.ui.scroll_offset == usize::MAX || app.ui.scroll_offset >= max_scroll {
            app.ui.scroll_offset = max_scroll;
            app.ui.auto_scroll = true;
        }
    } else if let Some(msg_start) = cached
        .msg_start_lines
        .iter()
        .find(|(idx, _)| *idx == app.ui.browse_msg_index)
        .map(|(_, line)| *line)
    {
        let msg_line_count = cached
            .per_msg_lines
            .get(app.ui.browse_msg_index)
            .map(|c| c.lines.len())
            .unwrap_or(1);
        let msg_max_scroll = msg_line_count.saturating_sub(visible_height);
        if app.ui.browse_scroll_offset > msg_max_scroll {
            app.ui.browse_scroll_offset = msg_max_scroll;
        }
        app.ui.scroll_offset = (msg_start + app.ui.browse_scroll_offset).min(max_scroll);
    }

    // 先清除 inner 区域的旧字符（reset 每个 cell 的 symbol 为空格），
    // 再用背景色填充。ratatui 的 Block/Paragraph 只调用 set_style（不改 symbol），
    // 在窄屏滚动时会导致右侧残留上一帧的字符。
    f.render_widget(ratatui::widgets::Clear, inner);
    let bg_fill = Block::default().style(Style::default().bg(app.ui.theme.bg_primary));
    f.render_widget(bg_fill, inner);

    // === 文字渲染 pass + 图片标记收集 ===
    let (start, img_markers) = {
        let cached = match app.ui.msg_lines_cache.as_ref() {
            Some(c) => c,
            None => return,
        };
        let start = app.ui.scroll_offset;
        let end = (start + visible_height).min(cached.total_line_count);
        let history_total = cached.history_line_count;
        let msg_area_bg = Style::default().bg(app.ui.theme.bg_primary);
        let selection = app.ui.mouse_selection.as_ref();
        let img_markers = render_text_pass(
            f,
            &TextPassParams {
                inner,
                cached,
                start,
                end,
                history_total,
                msg_area_bg,
            },
            selection,
        );
        (start, img_markers)
    }; // cached 借用在此释放

    // === 图片渲染 pass（需在文字之后覆盖绘制）===
    render_image_pass(
        f,
        inner,
        img_markers,
        start,
        visible_height as u16,
        bubble_max_width,
        app,
    );
}
