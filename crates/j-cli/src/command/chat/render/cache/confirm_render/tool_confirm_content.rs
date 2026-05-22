//! 工具确认内容渲染

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::app::{ChatApp, ToolCallStatus};
use crate::command::chat::constants::CONFIRM_MSG_MAX_LINES;
use crate::command::chat::render::cache::bubble::bordered_line;
use crate::util::text::{display_width, wrap_text};

/// 渲染工具确认模式的内容和选项
pub(crate) fn render_tool_confirm_content(
    app: &ChatApp,
    tc: &ToolCallStatus,
    bubble_max_width: usize,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let t = &app.ui.theme;
    let confirm_bg = t.tool_confirm_bg;
    let border_color = t.tool_confirm_border;

    // 工具名行（使用 bordered_line 确保溢出钳制）
    {
        lines.push(bordered_line(
            vec![
                Span::styled(" ", Style::default().bg(confirm_bg)),
                Span::styled(
                    "工具: ",
                    Style::default().fg(t.tool_confirm_label).bg(confirm_bg),
                ),
                Span::styled(
                    tc.tool_name.clone(),
                    Style::default()
                        .fg(t.tool_confirm_name)
                        .bg(confirm_bg)
                        .add_modifier(Modifier::BOLD),
                ),
            ],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

    // 确认信息行（折行显示，最多 CONFIRM_MSG_MAX_LINES 行，使用 bordered_line 确保溢出钳制）
    {
        let max_msg_w = content_w.saturating_sub(2);
        let wrapped = wrap_text(&tc.confirm_message, max_msg_w);
        let max_lines = CONFIRM_MSG_MAX_LINES;
        let show_lines = wrapped.len().min(max_lines);
        for (i, line_text) in wrapped.iter().enumerate().take(show_lines) {
            let display_text = if i == max_lines - 1 && wrapped.len() > max_lines {
                format!("{}...", line_text)
            } else {
                line_text.clone()
            };
            lines.push(bordered_line(
                vec![
                    Span::styled(" ", Style::default().bg(confirm_bg)),
                    Span::styled(
                        display_text,
                        Style::default().fg(t.tool_confirm_text).bg(confirm_bg),
                    ),
                ],
                bubble_max_width,
                border_color,
                confirm_bg,
            ));
        }
    }

    // 空行（使用 bordered_line 保持一致）
    {
        lines.push(bordered_line(
            vec![Span::styled(" ", Style::default().bg(confirm_bg))],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

    // 工具确认选项
    {
        let arrow_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let selected = app.ui.tool_interact_selected;

        let countdown_suffix = if app.state.agent_config.tool_confirm_timeout > 0 {
            let elapsed = app
                .tool_executor
                .tool_confirm_entered_at
                .elapsed()
                .as_secs();
            let remaining = app
                .state
                .agent_config
                .tool_confirm_timeout
                .saturating_sub(elapsed);
            format!(" ({}s)", remaining)
        } else {
            String::new()
        };
        let options: Vec<String> = vec![
            format!("continue: 确认执行{}", countdown_suffix),
            "allow: 允许并记住".to_string(),
            "refuse: 拒绝执行".to_string(),
            "type something...".to_string(),
        ];

        for (i, option) in options.iter().enumerate() {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯" } else { " " };

            if i == 3 && app.ui.tool_interact_typing {
                // 输入模式：显示带光标的输入框
                // 前缀 "❯ ✏ " 的显示宽度（与普通选项前缀 "❯ " 对齐，✏ 替代 type:）
                let prefix = "❯ ✏ ";
                let prefix_w = display_width(prefix);
                // 续行缩进宽度（与前缀对齐）
                let indent_w = prefix_w;
                let avail_w = content_w.saturating_sub(prefix_w);
                // 最少保证 4 列可用（光标占 1 + 至少 3 字符余量）
                let avail_w = avail_w.max(4);

                let input = &app.ui.tool_interact_input;
                let cursor_pos = app.ui.tool_interact_cursor;
                let chars: Vec<char> = input.chars().collect();
                let before: String = chars[..cursor_pos].iter().collect();
                let cursor_char = chars.get(cursor_pos).copied().unwrap_or(' ');
                let after: String = if cursor_pos < chars.len() {
                    chars[cursor_pos + 1..].iter().collect()
                } else {
                    String::new()
                };

                let text_style = Style::default().fg(t.text_white).bg(confirm_bg);
                let cursor_style = Style::default().fg(t.cursor_fg).bg(t.cursor_bg);
                let pointer_style = Style::default()
                    .fg(Color::Cyan)
                    .bg(confirm_bg)
                    .add_modifier(Modifier::BOLD);

                // 将 before / cursor_char / after 拼回完整文本，用 wrap_text 按可用宽度折行
                let full_text = format!("{}{}{}", before, cursor_char, after);
                let wrapped = wrap_text(&full_text, avail_w);

                // 定位光标所在折行：逐行累加宽度，找到 cursor_pos 落在哪一行
                let mut char_idx = 0usize;
                let mut cursor_line = 0usize;
                let mut cursor_offset_in_line = 0usize;
                for (li, line_str) in wrapped.iter().enumerate() {
                    let line_chars: Vec<char> = line_str.chars().collect();
                    if cursor_pos >= char_idx && cursor_pos < char_idx + line_chars.len() {
                        cursor_line = li;
                        cursor_offset_in_line = cursor_pos - char_idx;
                        break;
                    }
                    char_idx += line_chars.len();
                    if li == wrapped.len() - 1 && cursor_pos == char_idx {
                        // 光标在末尾
                        cursor_line = li;
                        cursor_offset_in_line = line_chars.len();
                    }
                }

                for (li, _line_str) in wrapped.iter().enumerate() {
                    let is_first = li == 0;
                    let prefix_span = if is_first {
                        Span::styled(prefix, pointer_style)
                    } else {
                        Span::styled(" ".repeat(indent_w), text_style)
                    };

                    if li == cursor_line {
                        // 光标行：需要拆分 before / cursor_char / after
                        let line_str = &wrapped[li];
                        let line_chars: Vec<char> = line_str.chars().collect();
                        let line_before: String =
                            line_chars[..cursor_offset_in_line].iter().collect();
                        let cc = line_chars
                            .get(cursor_offset_in_line)
                            .copied()
                            .unwrap_or(' ');
                        let line_after: String =
                            line_chars[cursor_offset_in_line + 1..].iter().collect();

                        lines.push(bordered_line(
                            vec![
                                prefix_span,
                                Span::styled(line_before, text_style),
                                Span::styled(cc.to_string(), cursor_style),
                                Span::styled(line_after, text_style),
                            ],
                            bubble_max_width,
                            border_color,
                            confirm_bg,
                        ));
                    } else {
                        // 非光标续行
                        lines.push(bordered_line(
                            vec![prefix_span, Span::styled(wrapped[li].clone(), text_style)],
                            bubble_max_width,
                            border_color,
                            confirm_bg,
                        ));
                    }
                }
            } else if i == 3 {
                // "type something..." 行：非输入状态下显示提示或预览
                let pointer_str = if is_selected { "❯ " } else { "  " };
                let pointer_style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(confirm_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().bg(confirm_bg)
                };
                // 显示已输入内容的预览或提示
                let input = &app.ui.tool_interact_input;
                let label_text = if input.is_empty() {
                    "✏ type something...".to_string()
                } else {
                    // 截断过长的预览
                    let max_preview = 30;
                    let preview: String = input.chars().take(max_preview).collect();
                    if input.chars().count() > max_preview {
                        format!("✏ 已输入: {}...", preview)
                    } else {
                        format!("✏ 已输入: {}", preview)
                    }
                };
                let text_style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(confirm_bg)
                        .add_modifier(Modifier::BOLD)
                } else if input.is_empty() {
                    Style::default().fg(t.tool_confirm_label).bg(confirm_bg)
                } else {
                    // 有输入内容时用稍亮的颜色提示
                    Style::default().fg(t.tool_confirm_label).bg(confirm_bg)
                };
                lines.push(bordered_line(
                    vec![
                        Span::styled(pointer_str, pointer_style),
                        Span::styled(label_text, text_style),
                    ],
                    bubble_max_width,
                    border_color,
                    confirm_bg,
                ));
            } else {
                // 非输入模式的普通选项行（使用 bordered_line 确保溢出钳制）
                let pointer_style = if is_selected {
                    arrow_style.bg(confirm_bg)
                } else {
                    Style::default().bg(confirm_bg)
                };
                let text_style = if is_selected {
                    arrow_style.bg(confirm_bg)
                } else {
                    Style::default().fg(t.tool_confirm_label).bg(confirm_bg)
                };
                lines.push(bordered_line(
                    vec![
                        Span::styled(pointer, pointer_style),
                        Span::styled(format!(" {}", option), text_style),
                    ],
                    bubble_max_width,
                    border_color,
                    confirm_bg,
                ));
            }
        }
    }
}
