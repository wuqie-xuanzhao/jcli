//! 工具结果渲染：入口函数、公共类型、辅助函数、小型渲染

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::constants::{ERROR_RESULT_MAX_LINES, NORMAL_RESULT_MAX_LINES};
use crate::command::chat::render::cache::msg_render::agent_name_color;
use crate::command::chat::render::cache::{ContentContext, TOOL_RESULT_DISPLAY_MAX_LINES};
use crate::command::chat::render::theme::Theme;
use crate::command::chat::render::theme::{ToolCategoryColor, ToolStatusColor};
use crate::command::chat::tools::classification::{
    ToolCategory, ToolStatus, get_result_summary_for_tool,
};
use crate::command::chat::tools::tool_names;
use crate::markdown::highlight::highlight_code_line;
use crate::tui::editor_core::EditorTheme;
use crate::util::text::{
    line_number_continuation_prefix, line_number_prefix_width, wrap_text, wrap_text_with_prefix,
};

// 导入子模块
mod file_operations;
mod helpers;
mod search_tools;
mod task_management;

// 公开导出子模块中需要外部使用的函数
pub(crate) use file_operations::render_glob_result;
pub(crate) use helpers::{render_agent_result_nested, render_bash_result, render_diff_content};
pub(crate) use search_tools::{
    render_grep_result, render_web_fetch_result, render_web_search_result,
};
pub(crate) use task_management::{render_task_output_result, render_task_result};

/// render_tool_result_msg 的只读参数（lines 作为输出单独传）
pub struct ToolResultRenderParams<'a> {
    pub sender_name: Option<&'a str>,
    pub content: &'a str,
    pub label: &'a str,
    pub tool_args: Option<&'a str>,
    pub bubble_max_width: usize,
    pub theme: &'a Theme,
    pub expand: bool,
}

/// 渲染工具执行结果消息：展开时完整内容，折叠时只显示标签
pub fn render_tool_result_msg(params: &ToolResultRenderParams, lines: &mut Vec<Line<'static>>) {
    // 解构出局部变量，保持函数体不变
    let sender_name = params.sender_name;
    let content = params.content;
    let label = params.label;
    let tool_args = params.tool_args;
    let bubble_max_width = params.bubble_max_width;
    let theme = params.theme;
    let expand = params.expand;
    // 构建首行前缀：有 sender_name 时 "  name · "，否则 "  "
    let sender_prefix_spans: Vec<Span<'static>> = if let Some(name) = sender_name {
        let label_color = agent_name_color(name);
        vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                name.to_string(),
                Style::default()
                    .fg(label_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" · ", Style::default().fg(theme.text_dim)),
        ]
    } else {
        vec![Span::styled("  ", Style::default())]
    };

    // 与前一条消息（tool_call）之间留一行间距
    lines.push(Line::from(""));

    // 解析 label，格式为 "工具名..." 或 "工具名[id]..."
    let (tool_name, is_error) = parse_tool_label(label);
    let category = ToolCategory::from_name(&tool_name);
    let tool_color = category.color(theme);
    // tool_result 统一使用 🔧 图标，与 tool_call_request 的分类图标区分
    let icon = "🔧";

    let status = if is_error {
        ToolStatus::Failed
    } else {
        ToolStatus::Success
    };
    let status_icon = status.icon();
    let status_color = status.color(theme);

    // 获取结果摘要
    let summary = get_result_summary_for_tool(content, is_error, &tool_name, tool_args);

    // 第一行：sender_prefix + 图标 + 工具名 + 状态 + 摘要
    let mut spans = sender_prefix_spans;
    spans.push(Span::styled(icon, Style::default().fg(tool_color)));
    spans.push(Span::styled(" ", Style::default()));
    spans.push(Span::styled(
        tool_name.clone(),
        Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(" ", Style::default()));
    spans.push(Span::styled(status_icon, Style::default().fg(status_color)));
    spans.push(Span::styled(" ", Style::default()));
    spans.push(Span::styled(summary, Style::default().fg(theme.text_dim)));
    lines.push(Line::from(spans));

    // Todo 工具特殊处理：折叠模式也显示 todo 列表
    let is_todo_tool = tool_name == "TodoRead" || tool_name == "TodoWrite";

    if (!expand && !is_todo_tool) || content.is_empty() {
        return;
    }

    // 展开模式：缩进显示内容
    let clean = crate::util::text::sanitize_tool_output(content);
    let content_w = bubble_max_width.saturating_sub(6);

    // 错误结果特殊处理
    if is_error {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(
                "Error:",
                Style::default()
                    .fg(theme.toast_error_border)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        let error_lines: Vec<&str> = clean.lines().take(ERROR_RESULT_MAX_LINES).collect();
        for line in error_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("      {}", wrapped),
                    Style::default().fg(theme.toast_error_border),
                )));
            }
        }

        let total_lines = clean.lines().count();
        let max_err_lines = ERROR_RESULT_MAX_LINES;
        if total_lines > max_err_lines {
            lines.push(Line::from(Span::styled(
                format!(
                    "    ... (共 {} 行，显示前 {} 行)",
                    total_lines, max_err_lines
                ),
                Style::default().fg(theme.text_dim),
            )));
        }
    } else if clean.contains("```diff\n") {
        // Diff 块特殊渲染
        render_diff_content(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::AGENT
        || tool_name == tool_names::TEAMMATE
        || tool_name == tool_names::COMPACT
        || tool_name == tool_names::LOAD_SKILL
        || tool_name == tool_names::ENTER_PLAN_MODE
        || tool_name == tool_names::EXIT_PLAN_MODE
    {
        // Agent/Compact/LoadSkill/Plan 结果边框显示
        render_agent_result_nested(&clean, bubble_max_width, lines, theme);
    } else if tool_name == tool_names::SHELL {
        // Bash 结果：命令行高亮 + 输出
        render_bash_result(
            &clean,
            tool_args,
            &mut ContentContext {
                content_w,
                lines,
                theme,
                expand: false,
            },
        );
    } else if tool_name == tool_names::TODO_READ || tool_name == tool_names::TODO_WRITE {
        // TodoRead/TodoWrite 结果：折叠和展开都显示 todo 列表
        render_todo_result(
            &clean,
            &mut ContentContext {
                content_w,
                lines,
                theme,
                expand,
            },
        );
    } else if tool_name == tool_names::READ {
        // Read 工具结果：带行号的代码，支持语法高亮
        let lang = tool_args
            .and_then(parse_file_path_from_json)
            .map(|p| infer_lang_from_path(&p))
            .unwrap_or("");
        let editor_theme = EditorTheme::from(theme);

        let all_lines: Vec<&str> = clean.lines().take(NORMAL_RESULT_MAX_LINES).collect();
        for line in all_lines {
            if let Some((prefix_w, cont_prefix)) = line_number_continuation_prefix(line) {
                // 分离行号前缀和代码内容
                let prefix_str: String = line.chars().take(prefix_w).collect();
                let code_content: String = line.chars().skip(prefix_w).collect();
                // 续行
                for (i, wrapped) in wrap_text_with_prefix(&code_content, content_w, &cont_prefix)
                    .into_iter()
                    .enumerate()
                {
                    let mut spans = vec![Span::styled("    ", Style::default().fg(theme.text_dim))];
                    if i == 0 {
                        // 馀行：行号前缀
                        spans.push(Span::styled(
                            prefix_str.clone(),
                            Style::default().fg(theme.text_dim),
                        ));
                    } else {
                        // 续行：缩进对齐
                        spans.push(Span::styled(
                            cont_prefix.clone(),
                            Style::default().fg(theme.text_dim),
                        ));
                    }
                    if lang.is_empty() {
                        spans.push(Span::styled(wrapped, Style::default().fg(theme.text_dim)));
                    } else {
                        spans.extend(highlight_code_line(&wrapped, lang, &editor_theme));
                    }
                    lines.push(Line::from(spans));
                }
            } else {
                // 无行号前缀的行（如空行）
                for wrapped in wrap_text(line, content_w) {
                    if lang.is_empty() {
                        lines.push(Line::from(Span::styled(
                            format!("    {}", wrapped),
                            Style::default().fg(theme.text_dim),
                        )));
                    } else {
                        let mut spans =
                            vec![Span::styled("    ", Style::default().fg(theme.text_dim))];
                        spans.extend(highlight_code_line(&wrapped, lang, &editor_theme));
                        lines.push(Line::from(spans));
                    }
                }
            }
        }

        let total_lines = clean.lines().count();
        if total_lines > TOOL_RESULT_DISPLAY_MAX_LINES {
            lines.push(Line::from(Span::styled(
                format!(
                    "    ... (共 {} 行，显示前 {} 行)",
                    total_lines, TOOL_RESULT_DISPLAY_MAX_LINES
                ),
                Style::default().fg(theme.text_dim),
            )));
        }
    } else if tool_name == tool_names::GLOB {
        // Glob 工具结果：树形文件列表
        render_glob_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::GREP {
        // Grep 工具结果：结构化搜索结果
        render_grep_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::WEB_SEARCH {
        // WebSearch 工具结果：结构化搜索结果
        render_web_search_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::WEB_FETCH {
        // WebFetch 工具结果：内容预览（可能含 Markdown）
        render_web_fetch_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::TASK {
        // Task 工具结果：任务列表
        render_task_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::WRITE || tool_name == tool_names::EDIT {
        // Write/Edit 工具结果：文件路径高亮
        render_write_edit_result(&clean, tool_args, content_w, lines, theme);
    } else if tool_name == tool_names::TASK_OUTPUT {
        // TaskOutput 工具结果：结构化任务输出
        render_task_output_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::SEND_MESSAGE {
        // SendMessage 工具结果：发送确认
        render_send_message_result(&clean, tool_args, lines, theme);
    } else {
        // 正常结果
        let all_lines: Vec<&str> = clean.lines().take(NORMAL_RESULT_MAX_LINES).collect();
        // 预扫描：判断是否大部分行都带行号前缀（如 Read 工具输出）
        let numbered_count = all_lines
            .iter()
            .filter(|l| line_number_prefix_width(l).is_some())
            .count();
        let mostly_numbered = !all_lines.is_empty() && numbered_count * 2 >= all_lines.len();

        for line in all_lines {
            if mostly_numbered {
                // 带行号的文本：续行保留 │ 符号，与内容列对齐
                let (_, cont_prefix) =
                    line_number_continuation_prefix(line).unwrap_or_else(|| (0, String::new()));
                for wrapped in wrap_text_with_prefix(line, content_w, &cont_prefix) {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", wrapped),
                        Style::default().fg(theme.text_dim),
                    )));
                }
            } else {
                // 普通文本：标准 wrap
                for wrapped in wrap_text(line, content_w) {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", wrapped),
                        Style::default().fg(theme.text_dim),
                    )));
                }
            }
        }

        let total_lines = clean.lines().count();
        if total_lines > TOOL_RESULT_DISPLAY_MAX_LINES {
            lines.push(Line::from(Span::styled(
                format!(
                    "    ... (共 {} 行，显示前 {} 行)",
                    total_lines, TOOL_RESULT_DISPLAY_MAX_LINES
                ),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}

/// 渲染 TodoRead/TodoWrite 工具结果（实心点/空心点样式）
/// expand=true 时额外显示完成/未完成条数统计
pub(crate) fn render_todo_result(content: &str, ctx: &mut ContentContext<'_>) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let content_w = ctx.content_w;
    let expand = ctx.expand;
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
        // 展开模式：先显示统计信息
        if expand {
            let total = items.len();
            let completed = items
                .iter()
                .filter(|i| i.get("status").and_then(|s| s.as_str()) == Some("completed"))
                .count();
            let pending = total.saturating_sub(completed);

            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("完成 {} / 未完成 {}", completed, pending),
                    Style::default().fg(theme.text_dim),
                ),
            ]));

            lines.push(Line::from(""));
        }

        // 列出每个 todo 项
        for item in &items {
            let status = item
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("pending");
            let text = item
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("(empty)");

            // 实心点 ● 表示已完成/进行中，空心点 ○ 表示未开始
            let (dot, color) = match status {
                "completed" => ("●", theme.label_ai),        // 绿色实心点
                "in_progress" => ("◉", theme.title_loading), // 黄色双圈实心点
                "cancelled" => ("◌", theme.text_dim),        // 灰色空心虚圈
                _ => ("○", Color::Yellow),                   // pending: 黄色空心点
            };

            let text_style = if status == "completed" {
                Style::default()
                    .fg(theme.text_dim)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default().fg(theme.text_white)
            };
            let max_w = content_w.saturating_sub(10); // "    ● " prefix
            for (i, wrapped) in wrap_text(text, max_w).iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(dot, Style::default().fg(color)),
                        Span::styled(" ", Style::default()),
                        Span::styled(wrapped.clone(), text_style),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled(wrapped.clone(), text_style),
                    ]));
                }
            }
        }
    } else {
        // 非 JSON，回退到普通显示
        let all_lines: Vec<&str> = content.lines().take(100).collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }
    }
}

/// SendMessage 结果：发送确认
fn render_send_message_result(
    content: &str,
    tool_args: Option<&str>,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    // 从 tool_args 提取目标
    let target = tool_args.and_then(|args| {
        let pattern = "\"to\"";
        if let Some(idx) = args.find(pattern) {
            let rest = &args[idx + pattern.len()..];
            let rest = rest.trim_start_matches([' ', ':', '\t']);
            if let Some(stripped) = rest.strip_prefix('"')
                && let Some(end) = stripped.find('"')
            {
                return Some(stripped[..end].to_string());
            }
        }
        None
    });

    // 从 tool_args 提取消息内容
    let message = tool_args.and_then(|args| {
        let pattern = "\"message\"";
        if let Some(idx) = args.find(pattern) {
            let rest = &args[idx + pattern.len()..];
            let rest = rest.trim_start_matches([' ', ':', '\t']);
            if let Some(stripped) = rest.strip_prefix('"')
                && let Some(end) = stripped.find('"')
            {
                return Some(stripped[..end].to_string());
            }
        }
        None
    });

    // 第一行：发送目标
    if let Some(t) = &target {
        lines.push(Line::from(vec![
            Span::styled("    -> ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("@{}", t),
                Style::default()
                    .fg(theme.config_title)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    // 消息预览
    if let Some(msg) = &message {
        for wrapped in wrap_text(msg, 80) {
            lines.push(Line::from(Span::styled(
                format!("       {}", wrapped),
                Style::default().fg(theme.text_dim),
            )));
        }
    } else {
        // 无消息预览，显示原始 content
        for line in content.lines().take(NORMAL_RESULT_MAX_LINES) {
            lines.push(Line::from(Span::styled(
                format!("    {}", line),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}

/// Write/Edit 结果：文件路径高亮（导出给 file_operations 模块使用）
pub(crate) fn render_write_edit_result(
    content: &str,
    tool_args: Option<&str>,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    file_operations::render_write_edit_result(content, tool_args, content_w, lines, theme)
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────

/// 解析工具标签，提取工具名和错误状态
pub(crate) fn parse_tool_label(label: &str) -> (String, bool) {
    let is_error = label.contains("错误") || label.contains("失败") || label.contains("error");
    // 兼容旧格式 "工具 xxx" 和新格式直接工具名
    let tool_name = if label.starts_with("工具 ") {
        label
            .chars()
            .skip(3)
            .collect::<String>()
            .split(['.', ' '])
            .next()
            .unwrap_or(label)
            .to_string()
    } else {
        label.split(['.', ' ']).next().unwrap_or(label).to_string()
    };
    (tool_name, is_error)
}

/// 从 tool_args JSON 中提取 file_path 字段
pub(crate) fn parse_file_path_from_json(json: &str) -> Option<String> {
    // 简单解析：查找 "file_path": "..." 或 "path": "..."
    let patterns = ["\"file_path\"", "\"path\""];
    for pattern in patterns {
        if let Some(idx) = json.find(pattern) {
            let rest = &json[idx + pattern.len()..];
            // 跳过可能的空白和冒号
            let rest = rest.trim_start_matches([' ', ':', '\t']);
            // 提取引号内的字符串
            if let Some(stripped) = rest.strip_prefix('"') {
                let end = stripped.find('"')?;
                return Some(stripped[..end].to_string());
            }
        }
    }
    None
}

/// 根据文件扩展名推断语法高亮语言
pub(crate) fn infer_lang_from_path(path: &str) -> &'static str {
    let path_lower = path.to_lowercase();
    // 先检查完整文件名（如 Makefile）
    if path_lower.ends_with("makefile") || path_lower.ends_with("justfile") {
        return "makefile";
    }
    if path_lower.ends_with("dockerfile") {
        return "dockerfile";
    }
    // 扩展名映射
    let ext = path_lower.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "go" => "go",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "java" => "java",
        "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "hh" => "cpp",
        "sh" | "bash" | "zsh" => "bash",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" | "markdown" => "markdown",
        "sql" => "sql",
        "css" | "scss" | "sass" => "css",
        "html" | "htm" => "html",
        "xml" => "xml",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "rb" => "ruby",
        "php" => "php",
        "lua" => "lua",
        "vim" => "vim",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        _ => "",
    }
}
