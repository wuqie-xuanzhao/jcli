//! 鼠标选区功能模块
//!
//! 提供屏幕坐标到文本位置的映射、选区文本提取和剪贴板复制功能。

use crate::command::chat::app::{ChatApp, MsgLinesCache};
use crate::command::chat::render::cache::copy_to_clipboard;
use crate::tui::components::selection::normalize_selection;
use crate::util::text::char_width;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
};

/// 给定全局行号，定位到 per_msg_lines 或 streaming_lines 中对应的行引用
/// history_total 是所有历史消息的总行数（预计算，避免重复求和）
pub(crate) fn get_line_at(
    cached: &MsgLinesCache,
    global_idx: usize,
    history_total: usize,
) -> Option<&Line<'static>> {
    if global_idx < history_total {
        // 二分查找 msg_start_lines 定位所属消息
        let msg_pos = cached
            .msg_start_lines
            .partition_point(|&(_, start)| start <= global_idx);
        if msg_pos == 0 {
            return None;
        }
        let (_msg_idx, start) = cached.msg_start_lines[msg_pos - 1];
        let local = global_idx - start;
        let per = &cached.per_msg_lines[msg_pos - 1];
        per.lines.get(local)
    } else {
        cached.streaming_lines.get(global_idx - history_total)
    }
}

// ========== 鼠标选区坐标映射 ==========

/// 将屏幕坐标转换为 (全局行号, 行内字符偏移)
/// 返回 None 表示点击在消息区域外、空白区域或不可选行（边框、label、空行等）
#[allow(clippy::too_many_arguments)]
pub fn screen_to_text_pos(
    screen_x: u16,
    screen_y: u16,
    inner: Rect,
    scroll_offset: usize,
    cached: &MsgLinesCache,
) -> Option<(usize, usize)> {
    // 1. 计算全局行号
    let local_y = screen_y.saturating_sub(inner.y);
    if local_y >= inner.height {
        return None;
    }
    let global_line = scroll_offset + local_y as usize;
    if global_line >= cached.total_line_count {
        return None;
    }

    // 2. 获取该行的 Line
    let history_total = cached.history_line_count;
    let line = get_line_at(cached, global_line, history_total)?;

    // 3. 检查该行是否可选（跳过边框、label、空行）
    if !is_selectable_line(line) {
        return None;
    }

    // 4. 计算行内字符偏移（考虑 CJK 宽字符）
    let local_x = screen_x.saturating_sub(inner.x) as usize;
    let char_offset = spans_to_char_offset(&line.spans, local_x);

    Some((global_line, char_offset))
}

/// 判断一个渲染行是否可选（即非边框、非空行、非 label）
/// 通过检查 spans 内容来区分：边框行只含空格和 box-drawing 字符
pub(crate) fn is_selectable_line(line: &Line<'static>) -> bool {
    let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    // 空行不可选
    if full_text.trim().is_empty() {
        return false;
    }
    // 纯边框行不可选（只含空格 + box-drawing 字符：╭╮╰╯│─┌┐└┘）
    let trimmed = full_text.trim();
    if trimmed
        .chars()
        .all(|c| "╭╮╰╯│─┌┐└┘┬┴┼┤├".contains(c) || c == ' ')
    {
        return false;
    }
    true
}

/// 判断一个 span 是否是装饰性的（边框、padding、图片标记）
pub(crate) fn is_decorative_span(span: &Span<'static>) -> bool {
    let content = span.content.as_ref();
    // 图片标记
    if content.starts_with("\x00IMG:") {
        return true;
    }
    // 纯空格（padding）
    if content.chars().all(|c| c == ' ') {
        return true;
    }
    // 纯 box-drawing 字符（边框）
    if content.chars().all(|c| "╭╮╰╯│─┌┐└┘┬┴┼┤├".contains(c)) {
        return true;
    }
    false
}

/// 从渲染行的 spans 中提取纯内容文本（去掉装饰 span）
/// 返回 (内容文本, 内容在渲染行中的起始字符偏移)
pub(crate) fn extract_content_from_line(line: &Line<'static>) -> (String, usize) {
    let mut content = String::new();
    let mut content_start_offset = 0usize;
    let mut in_content = false;

    for span in &line.spans {
        let span_chars = span.content.chars().count();
        if is_decorative_span(span) {
            if !in_content {
                // 还在内容之前的装饰区域
                content_start_offset += span_chars;
            }
            // 内容之后的装饰区域，忽略
        } else {
            // 内容 span
            if !in_content {
                in_content = true;
            }
            content.push_str(span.content.as_ref());
        }
    }

    (content, content_start_offset)
}

/// 根据 spans 和屏幕 x 坐标计算字符偏移
pub(crate) fn spans_to_char_offset(spans: &[Span<'static>], screen_col: usize) -> usize {
    let mut acc_width = 0usize;
    let mut char_offset = 0usize;

    for span in spans {
        for ch in span.content.chars() {
            let w = char_width(ch);
            if acc_width >= screen_col {
                return char_offset;
            }
            acc_width += w;
            char_offset += 1;
        }
    }
    char_offset
}

/// 根据选区范围，从渲染行中提取纯内容文本（去掉边框和 padding）。
/// anchor/current 的字符偏移是相对于渲染行的，会自动转换为内容偏移。
pub fn extract_selection_text(
    cached: &MsgLinesCache,
    anchor: (usize, usize),
    current: (usize, usize),
) -> String {
    let ((sr, sc), (er, ec)) = normalize_selection(anchor, current);
    let history_total = cached.history_line_count;

    let mut result = String::new();

    for gline in sr..=er {
        let line = match get_line_at(cached, gline, history_total) {
            Some(l) => l,
            None => continue,
        };

        // 跳过不可选行
        if !is_selectable_line(line) {
            continue;
        }

        // 提取纯内容文本和内容起始偏移
        let (content_text, content_start) = extract_content_from_line(line);
        if content_text.is_empty() {
            continue;
        }

        // 将渲染行偏移转换为内容偏移
        let render_start = if gline == sr { sc } else { 0 };
        let render_end = if gline == er { ec } else { usize::MAX };

        // 内容区域：[content_start, content_start + content_len)
        let content_len = content_text.chars().count();
        let content_end = content_start + content_len;

        // 计算交集：渲染选区 ∩ 内容区域
        let intersect_start = render_start.max(content_start);
        let intersect_end = if render_end == usize::MAX {
            content_end
        } else {
            render_end.min(content_end)
        };

        if intersect_start >= intersect_end {
            continue;
        }

        // 转为内容文本内的字符偏移
        let text_start = intersect_start - content_start;
        let text_end = intersect_end - content_start;

        let chars: Vec<char> = content_text.chars().collect();
        let text_end = text_end.min(chars.len());
        if text_start < text_end {
            let slice: String = chars[text_start..text_end].iter().collect();
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&slice);
        }
    }

    result
}

/// 复制选区文本到剪贴板，并显示 toast 提示
pub fn copy_selection_to_clipboard(app: &mut ChatApp) {
    let cached = match app.ui.msg_lines_cache.as_ref() {
        Some(c) => c,
        None => return,
    };

    let sel = match &app.ui.mouse_selection {
        Some(s) => s,
        None => return,
    };

    let text = extract_selection_text(cached, sel.anchor, sel.current);
    if text.is_empty() {
        return;
    }

    if copy_to_clipboard(&text) {
        app.show_toast("已复制到剪贴板", false);
    } else {
        app.show_toast("复制到剪贴板失败", true);
    }
}
