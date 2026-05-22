//! 其他小型工具渲染模块
//!
//! 包含 Ask、Todo、Compact、PlanMode、LoadSkill、RegisterHook、SendMessage、
//! Worktree、WorkDone、IgnoreMessage、ComputerUse 等工具的专用渲染函数

use crate::command::chat::render::theme::Theme;
use crate::command::chat::tools::tool_names;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::{render_kv_line, render_tag_line, truncate_str};

// ──────────────────────────────────────────────────────────────
// Ask
// ──────────────────────────────────────────────────────────────

/// Ask 工具展开渲染
pub(crate) fn render_ask_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(questions) = parsed.get("questions").and_then(|v| v.as_array()) {
        for (i, q) in questions.iter().enumerate() {
            let question_text = q.get("question").and_then(|v| v.as_str()).unwrap_or("?");
            let header = q
                .get("header")
                .and_then(|v| v.as_str())
                .unwrap_or("question");

            // 问题标签
            let label = if questions.len() > 1 {
                format!("Q{} [{}]", i + 1, header)
            } else {
                header.to_string()
            };
            render_kv_line(&label, question_text, content_w, lines, theme);

            // 选项预览
            if let Some(options) = q.get("options").and_then(|v| v.as_array()) {
                let opts_preview: Vec<String> = options
                    .iter()
                    .filter_map(|o| o.get("label").and_then(|l| l.as_str()).map(String::from))
                    .collect();
                if !opts_preview.is_empty() {
                    render_kv_line(
                        "options",
                        &opts_preview.join(" / "),
                        content_w,
                        lines,
                        theme,
                    );
                }
            }
        }
    }

    true
}

// ──────────────────────────────────────────────────────────────
// Todo
// ──────────────────────────────────────────────────────────────

/// TodoWrite 工具展开渲染
pub(crate) fn render_todo_write_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(todos) = parsed.get("todos").and_then(|v| v.as_array()) {
        render_tag_line(
            &format!("待办列表 ({} 项)", todos.len()),
            content_w,
            lines,
            theme,
        );
        for todo in todos {
            let content = todo.get("content").and_then(|v| v.as_str()).unwrap_or("?");
            let status = todo
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");
            let bullet = match status {
                "completed" => "[x]",
                "in_progress" => "[~]",
                "cancelled" => "[-]",
                _ => "[ ]",
            };
            let line_text = format!("{} {}", bullet, content);
            let display = truncate_str(&line_text, content_w);
            lines.push(Line::from(vec![
                Span::styled("      ", Style::default()),
                Span::styled(display, Style::default().fg(theme.text_dim)),
            ]));
        }
    }

    true
}

/// TodoRead 工具展开渲染
pub(crate) fn render_todo_read_call_request_expanded(
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    render_tag_line("读取待办列表", content_w, lines, theme);
    true
}

// ──────────────────────────────────────────────────────────────
// Compact
// ──────────────────────────────────────────────────────────────

/// Compact 工具展开渲染
pub(crate) fn render_compact_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    render_tag_line("压缩对话上下文", content_w, lines, theme);

    if let Some(focus) = parsed.get("focus").and_then(|v| v.as_str()) {
        render_kv_line("focus", focus, content_w, lines, theme);
    }

    true
}

// ──────────────────────────────────────────────────────────────
// PlanMode
// ──────────────────────────────────────────────────────────────

/// EnterPlanMode 工具展开渲染
pub(crate) fn render_enter_plan_mode_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    render_tag_line("进入计划模式（只读模式）", content_w, lines, theme);

    if let Some(desc) = parsed.get("description").and_then(|v| v.as_str()) {
        render_kv_line("plan", desc, content_w, lines, theme);
    }

    true
}

// ──────────────────────────────────────────────────────────────
// LoadSkill
// ──────────────────────────────────────────────────────────────

/// LoadSkill 工具展开渲染
pub(crate) fn render_load_skill_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let name = parsed
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    render_tag_line(&format!("加载技能: {}", name), content_w, lines, theme);

    if let Some(args) = parsed.get("arguments").and_then(|v| v.as_str()) {
        render_kv_line("arguments", args, content_w, lines, theme);
    }

    true
}

// ──────────────────────────────────────────────────────────────
// RegisterHook
// ──────────────────────────────────────────────────────────────

/// RegisterHook 工具展开渲染
pub(crate) fn render_register_hook_call_request_expanded(
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
        .unwrap_or("register");
    render_tag_line(&format!("[{}]", action), content_w, lines, theme);

    if let Some(event) = parsed.get("event").and_then(|v| v.as_str()) {
        render_kv_line("event", event, content_w, lines, theme);
    }

    if let Some(hook_type) = parsed.get("type").and_then(|v| v.as_str()) {
        render_kv_line("type", hook_type, content_w, lines, theme);
    }

    if let Some(command) = parsed.get("command").and_then(|v| v.as_str()) {
        render_kv_line("command", command, content_w, lines, theme);
    }

    if let Some(prompt) = parsed.get("prompt").and_then(|v| v.as_str()) {
        render_kv_line(
            "prompt",
            &truncate_str(prompt, 100),
            content_w,
            lines,
            theme,
        );
    }

    true
}

// ──────────────────────────────────────────────────────────────
// SendMessage
// ──────────────────────────────────────────────────────────────

/// SendMessage 工具展开渲染
pub(crate) fn render_send_message_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let to = parsed.get("to").and_then(|v| v.as_str());
    if let Some(target) = to {
        render_kv_line("to", &format!("@{}", target), content_w, lines, theme);
    } else {
        render_tag_line("广播消息", content_w, lines, theme);
    }

    if let Some(message) = parsed.get("message").and_then(|v| v.as_str()) {
        render_kv_line(
            "message",
            &truncate_str(message, 100),
            content_w,
            lines,
            theme,
        );
    }

    true
}

// ──────────────────────────────────────────────────────────────
// Worktree
// ──────────────────────────────────────────────────────────────

/// EnterWorktree / ExitWorktree 工具展开渲染
pub(crate) fn render_worktree_call_request_expanded(
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

    if tool_name == tool_names::ENTER_WORKTREE {
        render_tag_line("进入隔离工作树", content_w, lines, theme);
        if let Some(name) = parsed.get("name").and_then(|v| v.as_str()) {
            render_kv_line("name", name, content_w, lines, theme);
        }
    } else {
        let action = parsed
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("keep");
        render_tag_line("退出工作树", content_w, lines, theme);
        render_kv_line("action", action, content_w, lines, theme);
    }

    true
}

// ──────────────────────────────────────────────────────────────
// WorkDone
// ──────────────────────────────────────────────────────────────

/// WorkDone 工具展开渲染
pub(crate) fn render_work_done_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    render_tag_line("工作完成声明", content_w, lines, theme);

    if let Some(summary) = parsed.get("summary").and_then(|v| v.as_str()) {
        render_kv_line("summary", summary, content_w, lines, theme);
    }

    true
}

// ──────────────────────────────────────────────────────────────
// IgnoreMessage
// ──────────────────────────────────────────────────────────────

/// IgnoreMessage 工具展开渲染
pub(crate) fn render_ignore_message_call_request_expanded(
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    render_tag_line("忽略消息", content_w, lines, theme);
    true
}

// ──────────────────────────────────────────────────────────────
// ComputerUse (macOS only)
// ──────────────────────────────────────────────────────────────

/// ComputerUse 工具展开渲染
#[cfg(target_os = "macos")]
pub(crate) fn render_computer_use_call_request_expanded(
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

    if let Some(display_num) = parsed.get("display_number").and_then(|v| v.as_u64()) {
        render_kv_line("display", &display_num.to_string(), content_w, lines, theme);
    }

    true
}
