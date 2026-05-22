//! Task/Web 工具渲染模块
//!
//! 包含 Task、TaskOutput、WebSearch、WebFetch、Browser 工具的专用渲染函数
//! 渲染模式统一使用 render_kv_line / render_tag_line

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::{render_kv_line, render_tag_line};

// ──────────────────────────────────────────────────────────────
// Task
// ──────────────────────────────────────────────────────────────

/// Task 工具展开渲染
pub(crate) fn render_task_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let action = parsed
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("task");

    // action 标签
    render_tag_line(&format!("[{}]", action), content_w, lines, theme);

    // title
    if let Some(title) = parsed.get("title").and_then(|v| v.as_str()) {
        render_kv_line("title", title, content_w, lines, theme);
    }

    // description
    if let Some(desc) = parsed.get("description").and_then(|v| v.as_str()) {
        render_kv_line("description", desc, content_w, lines, theme);
    }

    // taskId（update/get 时）
    if let Some(task_id) = parsed.get("taskId").and_then(|v| v.as_str()) {
        render_kv_line("taskId", task_id, content_w, lines, theme);
    }

    // status（update 时）
    if let Some(status) = parsed.get("status").and_then(|v| v.as_str()) {
        render_kv_line("status", status, content_w, lines, theme);
    }

    true
}

// ──────────────────────────────────────────────────────────────
// TaskOutput
// ──────────────────────────────────────────────────────────────

/// TaskOutput 工具展开渲染
pub(crate) fn render_task_output_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(task_id) = parsed.get("task_id").and_then(|v| v.as_str()) {
        render_kv_line("task_id", task_id, content_w, lines, theme);
    }

    // block
    if let Some(block) = parsed.get("block").and_then(|v| v.as_bool()) {
        render_kv_line(
            "block",
            if block {
                "true (等待完成)"
            } else {
                "false (非阻塞)"
            },
            content_w,
            lines,
            theme,
        );
    }

    // timeout
    if let Some(timeout) = parsed.get("timeout").and_then(|v| v.as_u64()) {
        render_kv_line(
            "timeout",
            &format!("{}ms", timeout),
            content_w,
            lines,
            theme,
        );
    }

    true
}

// ──────────────────────────────────────────────────────────────
// WebSearch
// ──────────────────────────────────────────────────────────────

/// WebSearch 工具展开渲染
pub(crate) fn render_web_search_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(query) = parsed.get("query").and_then(|v| v.as_str()) {
        render_kv_line("query", query, content_w, lines, theme);
    }

    if let Some(count) = parsed.get("count").and_then(|v| v.as_u64()) {
        render_kv_line("count", &count.to_string(), content_w, lines, theme);
    }

    if let Some(search_type) = parsed.get("type").and_then(|v| v.as_str()) {
        render_kv_line("type", search_type, content_w, lines, theme);
    }

    true
}

// ──────────────────────────────────────────────────────────────
// WebFetch
// ──────────────────────────────────────────────────────────────

/// WebFetch 工具展开渲染
pub(crate) fn render_web_fetch_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(url) = parsed.get("url").and_then(|v| v.as_str()) {
        render_kv_line("url", url, content_w, lines, theme);
    }

    if let Some(mode) = parsed.get("extract_mode").and_then(|v| v.as_str()) {
        render_kv_line("mode", mode, content_w, lines, theme);
    }

    true
}

// ──────────────────────────────────────────────────────────────
// Browser
// ──────────────────────────────────────────────────────────────

/// Browser 工具展开渲染
pub(crate) fn render_browser_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(action) = parsed.get("action").and_then(|v| v.as_str()) {
        render_kv_line("action", action, content_w, lines, theme);
    }

    if let Some(url) = parsed.get("url").and_then(|v| v.as_str()) {
        render_kv_line("url", url, content_w, lines, theme);
    }

    true
}
