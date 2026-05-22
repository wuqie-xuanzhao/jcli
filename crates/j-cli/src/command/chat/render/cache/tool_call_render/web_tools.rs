//! Web 工具渲染模块
//!
//! 包含 WebSearch、WebFetch、Browser 工具的专用渲染函数

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::render_kv_line;

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
