//! 文件工具渲染模块
//!
//! 包含 Glob、Grep、Read、Write、Edit 工具的专用渲染函数

use crate::command::chat::render::theme::Theme;
use crate::command::chat::tools::tool_names;
use ratatui::text::Line;

use super::render_kv_line;

// ──────────────────────────────────────────────────────────────
// Glob/Grep
// ──────────────────────────────────────────────────────────────

/// Glob/Grep 工具展开渲染：只显示关键参数（path + pattern）
pub(crate) fn render_glob_grep_call_request_expanded(
    tool_name: &str,
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // path（截断显示）
    if let Some(path) = parsed.get("path").and_then(|v| v.as_str()) {
        let display_path = truncate_path(path, 60);
        render_kv_line("path", &display_path, content_w, lines, theme);
    }

    // pattern（仅 Grep）
    if tool_name == tool_names::GREP {
        if let Some(pattern) = parsed.get("pattern").and_then(|v| v.as_str()) {
            render_kv_line("pattern", pattern, content_w, lines, theme);
        }

        // output_mode
        if let Some(mode) = parsed.get("output_mode").and_then(|v| v.as_str())
            && mode != "content"
        {
            render_kv_line("mode", mode, content_w, lines, theme);
        }

        // context（若有）
        if let Some(ctx) = parsed.get("context").and_then(|v| v.as_u64())
            && ctx > 0
        {
            render_kv_line("context", &format!("{} 行", ctx), content_w, lines, theme);
        }
    }

    true
}

// ──────────────────────────────────────────────────────────────
// Read/Write/Edit
// ──────────────────────────────────────────────────────────────

/// Read/Write/Edit 工具展开渲染
pub(crate) fn render_file_tool_call_request_expanded(
    tool_name: &str,
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // path / file_path
    let path = parsed
        .get("path")
        .or_else(|| parsed.get("file_path"))
        .and_then(|v| v.as_str());

    if let Some(path) = path {
        let display_path = truncate_path(path, 60);
        render_kv_line("path", &display_path, content_w, lines, theme);
    }

    // Read: offset/limit（若有）
    if tool_name == tool_names::READ {
        if let Some(offset) = parsed.get("offset").and_then(|v| v.as_u64())
            && offset > 0
        {
            render_kv_line("offset", &format!("行 {}", offset), content_w, lines, theme);
        }
        if let Some(limit) = parsed.get("limit").and_then(|v| v.as_u64())
            && limit > 0
        {
            render_kv_line("limit", &format!("{} 行", limit), content_w, lines, theme);
        }
    }

    // Edit: old_string/new_string 摘要
    if tool_name == tool_names::EDIT {
        if let Some(old) = parsed.get("old_string").and_then(|v| v.as_str()) {
            let summary = summarize_string_content(old, 40);
            render_kv_line("old", &summary, content_w, lines, theme);
        }
        if let Some(new) = parsed.get("new_string").and_then(|v| v.as_str()) {
            let summary = summarize_string_content(new, 40);
            render_kv_line("new", &summary, content_w, lines, theme);
        }
        // replace_all
        if let Some(replace_all) = parsed.get("replace_all").and_then(|v| v.as_bool())
            && replace_all
        {
            render_kv_line("mode", "全部替换", content_w, lines, theme);
        }
    }

    true
}

// ──────────────────────────────────────────────────────────────
// Helper Functions
// ──────────────────────────────────────────────────────────────

/// 截断长路径（保留首尾）
pub(crate) fn truncate_path(path: &str, max_len: usize) -> String {
    if path.chars().count() > max_len {
        let first_part: String = path.chars().take(30).collect();
        let last_part: String = path.chars().rev().take(25).collect();
        format!(
            "{}...{}",
            first_part,
            last_part.chars().rev().collect::<String>()
        )
    } else {
        path.to_string()
    }
}

/// 摘要字符串内容（多行显示行数 + 首行预览，单行显示长度 + 截断预览）
pub(crate) fn summarize_string_content(s: &str, preview_len: usize) -> String {
    let line_count = s.lines().count();
    if line_count > 1 {
        // 多行：显示行数 + 首行预览
        let first_line = s.lines().next().unwrap_or("");
        let preview = if first_line.chars().count() > preview_len {
            format!(
                "{}...",
                first_line.chars().take(preview_len).collect::<String>()
            )
        } else {
            first_line.to_string()
        };
        format!("{} 行: \"{}\"", line_count, preview)
    } else {
        // 单行：显示长度 + 截断预览
        let char_count = s.chars().count();
        let preview: String = if char_count > preview_len {
            format!("{}...", s.chars().take(preview_len).collect::<String>())
        } else if char_count == 0 {
            String::from("(空)")
        } else {
            s.to_string()
        };
        if char_count > preview_len {
            format!("{} 字符: \"{}\"", char_count, preview)
        } else {
            format!("\"{}\"", preview)
        }
    }
}
