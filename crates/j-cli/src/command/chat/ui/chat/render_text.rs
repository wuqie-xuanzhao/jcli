//! 文字渲染 pass 模块
//!
//! 遍历可见行渲染文字，同时收集图片标记供后续图片渲染 pass 使用。

use super::selection::is_selectable_line;
use crate::command::chat::app::{MouseSelection, MsgLinesCache};
use crate::tui::components::selection::{
    compute_line_selection_range, rebuild_spans_with_selection,
};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// render_text_pass 的渲染参数（f 单独传）
pub(crate) struct TextPassParams<'a> {
    pub(crate) inner: Rect,
    pub(crate) cached: &'a MsgLinesCache,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) history_total: usize,
    pub(crate) msg_area_bg: Style,
}

/// 文字渲染 pass：遍历可见行渲染文字，同时收集图片标记。
/// 返回 `img_markers`: `(display_row, height, url)` 列表，供后续图片渲染 pass 使用。
/// P1 优化：通过消息范围预计算，避免逐行二分查找，只遍历可见消息。
pub(crate) fn render_text_pass(
    f: &mut ratatui::Frame,
    params: &TextPassParams,
    selection: Option<&MouseSelection>,
) -> Vec<(usize, u16, String)> {
    let mut img_markers: Vec<(usize, u16, String)> = Vec::new();
    let cached = params.cached;
    let history_total = params.history_total;

    // ★ P1 优化：使用二分查找定位第一条可见消息，然后顺序遍历
    // 只遍历 [start, end) 范围内涉及的 per_msg_lines 和 streaming_lines
    let visible_start = params.start;
    let visible_end = params.end;

    // 预计算历史消息的范围
    if visible_start < history_total && !cached.per_msg_lines.is_empty() {
        // 二分查找第一条可见消息
        let first_msg_pos = cached
            .msg_start_lines
            .partition_point(|&(_, start)| start <= visible_start)
            .saturating_sub(1);
        let first_msg_start = cached.msg_start_lines[first_msg_pos].1;

        // 顺序遍历消息，直到超出可见范围
        let mut line_idx = first_msg_start;
        for msg_pos in first_msg_pos..cached.per_msg_lines.len() {
            let per = &cached.per_msg_lines[msg_pos];
            let msg_line_count = per.lines.len();

            // 此消息的所有行
            for local in 0..msg_line_count {
                if line_idx >= visible_end {
                    break;
                }
                if line_idx >= visible_start {
                    let screen_i = line_idx - visible_start;
                    let y = params.inner.y + screen_i as u16;
                    let line_area = Rect::new(params.inner.x, y, params.inner.width, 1);
                    let line = &per.lines[local];

                    render_single_line(
                        f,
                        line,
                        line_area,
                        line_idx,
                        selection,
                        params.msg_area_bg,
                        &mut img_markers,
                        screen_i,
                    );
                }
                line_idx += 1;
            }
            if line_idx >= visible_end {
                break;
            }
        }
    }

    // 流式内容部分
    if visible_end > history_total {
        let stream_start = visible_start.saturating_sub(history_total);
        let stream_end = visible_end - history_total;
        for (local, line) in cached
            .streaming_lines
            .iter()
            .enumerate()
            .take(stream_end)
            .skip(stream_start)
        {
            let screen_i = history_total + local - visible_start;
            if screen_i >= visible_end - visible_start {
                break;
            }
            let y = params.inner.y + screen_i as u16;
            let line_area = Rect::new(params.inner.x, y, params.inner.width, 1);
            let global_idx = history_total + local;

            render_single_line(
                f,
                line,
                line_area,
                global_idx,
                selection,
                params.msg_area_bg,
                &mut img_markers,
                screen_i,
            );
        }
    }

    img_markers
}

/// 渲染单行（处理图片标记、选区高亮等）
#[allow(clippy::too_many_arguments)]
fn render_single_line(
    f: &mut ratatui::Frame,
    line: &Line<'static>,
    line_area: Rect,
    line_idx: usize,
    selection: Option<&MouseSelection>,
    msg_area_bg: Style,
    img_markers: &mut Vec<(usize, u16, String)>,
    screen_i: usize,
) {
    // 检查是否有图片标记 span
    let img_info: Option<(u16, String)> = line.spans.iter().find_map(|span| {
        span.content.strip_prefix("\x00IMG:").and_then(|rest| {
            rest.find(':').map(|p| {
                let height: u16 = rest[..p].parse().unwrap_or(20);
                let url = rest[p + 1..].to_string();
                (height, url)
            })
        })
    });

    if let Some((height, url)) = img_info {
        let visible_spans: Vec<Span> = line
            .spans
            .iter()
            .filter(|s| !s.content.starts_with("\x00IMG:"))
            .cloned()
            .collect();
        let p = Paragraph::new(Line::from(visible_spans)).style(msg_area_bg);
        f.render_widget(p, line_area);
        img_markers.push((screen_i, height, url));
    } else if let Some(sel) = selection
        && is_selectable_line(line)
    {
        let (sel_start, sel_end) = compute_line_selection_range(line_idx, sel.anchor, sel.current);
        if sel_start < sel_end {
            let fg = msg_area_bg.fg.unwrap_or(Color::White);
            let highlighted_spans = rebuild_spans_with_selection(
                &line.spans,
                0,
                sel_start,
                sel_end,
                fg,
                Color::DarkGray,
            );
            let p = Paragraph::new(Line::from(highlighted_spans)).style(msg_area_bg);
            f.render_widget(p, line_area);
        } else {
            let p = Paragraph::new(line.clone()).style(msg_area_bg);
            f.render_widget(p, line_area);
        }
    } else {
        let p = Paragraph::new(line.clone()).style(msg_area_bg);
        f.render_widget(p, line_area);
    }
}
