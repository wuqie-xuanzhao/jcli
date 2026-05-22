//! Ask 模式结构化问答渲染

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::app::ChatApp;
use crate::command::chat::render::cache::bubble::bordered_line;
use crate::markdown::markdown_to_lines;
use crate::util::text::{display_width, wrap_text};

/// 渲染 Ask 模式的结构化问答内容
pub(crate) fn render_ask_questions(
    app: &ChatApp,
    bubble_max_width: usize,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let t = &app.ui.theme;
    let confirm_bg = t.tool_confirm_bg;
    let border_color = t.tool_confirm_border;

    if let Some(cur_q) = app.ui.tool_ask_questions.get(app.ui.tool_ask_current_idx) {
        let total_q = app.ui.tool_ask_questions.len();
        let cur_idx = app.ui.tool_ask_current_idx;

        // header 标签 + 进度（过长时折行）
        let header_text = if total_q > 1 {
            format!("[{}/{}] {}", cur_idx + 1, total_q, cur_q.header)
        } else {
            cur_q.header.clone()
        };
        {
            // " " 前缀占 1 列，右侧留 1 列 padding
            let header_avail_w = content_w.saturating_sub(2).max(4);
            let header_wrapped = wrap_text(&header_text, header_avail_w);
            for hl in &header_wrapped {
                lines.push(bordered_line(
                    vec![Span::styled(
                        format!(" {}", hl),
                        Style::default().fg(t.tool_confirm_text).bg(confirm_bg),
                    )],
                    bubble_max_width,
                    border_color,
                    confirm_bg,
                ));
            }
        }

        // question 内容（Markdown 渲染）
        {
            let max_msg_w = content_w.saturating_sub(2);
            let md_lines_rendered = markdown_to_lines(&cur_q.question, max_msg_w, t);
            for md_line in md_lines_rendered.iter() {
                let is_img_marker = md_line
                    .spans
                    .iter()
                    .any(|s| s.content.starts_with("\x00IMG:"));
                let is_placeholder = md_line.spans.is_empty()
                    || md_line.spans.iter().all(|s| s.content.trim().is_empty());

                if is_img_marker {
                    let marker = match md_line
                        .spans
                        .iter()
                        .find(|s| s.content.starts_with("\x00IMG:"))
                    {
                        Some(s) => s.content.clone(),
                        None => continue,
                    };
                    let inner_w = bubble_max_width.saturating_sub(8);
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(border_color).bg(confirm_bg)),
                        Span::styled(" ".repeat(inner_w), Style::default().bg(confirm_bg)),
                        Span::styled(" │", Style::default().fg(border_color).bg(confirm_bg)),
                        Span::styled(marker, Style::default()),
                    ]));
                } else if is_placeholder {
                    // 空行
                    let inner_w = bubble_max_width.saturating_sub(4);
                    lines.push(Line::from(vec![
                        Span::styled("  │", Style::default().fg(border_color).bg(confirm_bg)),
                        Span::styled(" ".repeat(inner_w), Style::default().bg(confirm_bg)),
                        Span::styled("│", Style::default().fg(border_color).bg(confirm_bg)),
                    ]));
                } else {
                    let mut content_spans =
                        vec![Span::styled(" ", Style::default().bg(confirm_bg))];
                    for span in &md_line.spans {
                        let mut patched = span.clone();
                        patched.style = patched.style.bg(confirm_bg);
                        content_spans.push(patched);
                    }
                    lines.push(bordered_line(
                        content_spans,
                        bubble_max_width,
                        border_color,
                        confirm_bg,
                    ));
                }
            }
        }

        // 空行分隔
        {
            let inner_w = bubble_max_width.saturating_sub(4);
            lines.push(Line::from(vec![
                Span::styled("  │", Style::default().fg(border_color).bg(confirm_bg)),
                Span::styled(" ".repeat(inner_w), Style::default().bg(confirm_bg)),
                Span::styled("│", Style::default().fg(border_color).bg(confirm_bg)),
            ]));
        }

        // 渲染选项列表
        let is_multi = cur_q.multi_select;

        for (i, opt) in cur_q.options.iter().enumerate() {
            let is_cursor = i == app.ui.tool_ask_cursor;
            let is_selected_multi =
                i < app.ui.tool_ask_selections.len() && app.ui.tool_ask_selections[i];

            // 指示器和复选框用多个 span 实现颜色区分
            let pointer_str = if is_cursor { " ❯ " } else { "   " };
            let check_str = if is_multi {
                if is_selected_multi { "◉ " } else { "○ " }
            } else if is_cursor {
                "● "
            } else {
                "○ "
            };

            let pointer_style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .bg(confirm_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().bg(confirm_bg)
            };
            let check_style = if is_cursor || is_selected_multi {
                Style::default()
                    .fg(Color::Green)
                    .bg(confirm_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.tool_confirm_label).bg(confirm_bg)
            };
            let label_style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .bg(confirm_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.tool_confirm_label).bg(confirm_bg)
            };

            // label 折行：pointer + check 占去一段前缀，label 在剩余宽度内自动折行
            {
                let prefix_w = display_width(pointer_str) + display_width(check_str);
                let label_avail_w = content_w.saturating_sub(prefix_w + 2).max(4);
                let label_wrapped = wrap_text(&opt.label, label_avail_w);
                let indent_str = " ".repeat(prefix_w);
                for (li, label_line) in label_wrapped.iter().enumerate() {
                    if li == 0 {
                        lines.push(bordered_line(
                            vec![
                                Span::styled(pointer_str, pointer_style),
                                Span::styled(check_str, check_style),
                                Span::styled(label_line.clone(), label_style),
                            ],
                            bubble_max_width,
                            border_color,
                            confirm_bg,
                        ));
                    } else {
                        // 续行缩进对齐 label 起始列
                        lines.push(bordered_line(
                            vec![
                                Span::styled(indent_str.clone(), Style::default().bg(confirm_bg)),
                                Span::styled(label_line.clone(), label_style),
                            ],
                            bubble_max_width,
                            border_color,
                            confirm_bg,
                        ));
                    }
                }
            }

            // description 行（缩进，灰色）
            if !opt.description.is_empty() {
                let desc_prefix = "       ";
                let desc_max_w = content_w.saturating_sub(display_width(desc_prefix) + 2);
                let desc_wrapped = wrap_text(&opt.description, desc_max_w);
                for dl in &desc_wrapped {
                    let desc_text = format!("{}{}", desc_prefix, dl);
                    lines.push(bordered_line(
                        vec![Span::styled(
                            desc_text,
                            Style::default().fg(t.text_dim).bg(confirm_bg),
                        )],
                        bubble_max_width,
                        border_color,
                        confirm_bg,
                    ));
                }
            }
        }

        // "自由输入" 选项
        {
            let free_idx = cur_q.options.len();
            let is_cursor = free_idx == app.ui.tool_ask_cursor;

            if app.ui.tool_interact_typing {
                let pointer_style = Style::default()
                    .fg(Color::Cyan)
                    .bg(confirm_bg)
                    .add_modifier(Modifier::BOLD);

                // 块状光标渲染
                let input = &app.ui.tool_interact_input;
                let cursor_pos = app.ui.tool_interact_cursor;
                let chars: Vec<char> = input.chars().collect();

                // 光标前的文本
                let before: String = chars[..cursor_pos].iter().collect();
                // 光标处的字符（如果没有则使用空格）
                let cursor_char = chars.get(cursor_pos).copied().unwrap_or(' ');
                // 光标后的文本（光标位置+1 开始）
                let after: String = if cursor_pos < chars.len() {
                    chars[cursor_pos + 1..].iter().collect()
                } else {
                    String::new()
                };

                // 普通文本样式
                let text_style = Style::default().fg(t.text_white).bg(confirm_bg);
                // 块状光标样式（使用主题定义的光标颜色）
                let cursor_style = Style::default().fg(t.cursor_fg).bg(t.cursor_bg);

                // 前缀 " ❯ ✏ " 的显示宽度
                let prefix = " ❯ ✏ ";
                let prefix_w = display_width(prefix);
                // 续行缩进宽度（与前缀对齐）
                let indent_w = prefix_w;
                let avail_w = content_w.saturating_sub(prefix_w);
                // 最少保证 4 列可用（光标占 1 + 至少 3 字符余量）
                let avail_w = avail_w.max(4);

                // 拼回完整文本，用 wrap_text 按可用宽度折行
                let full_text = format!("{}{}{}", before, cursor_char, after);
                let wrapped = wrap_text(&full_text, avail_w);

                // 定位光标所在折行：逐行累加字符数，找到 cursor_pos 落在哪一行
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
            } else {
                let pointer_str = if is_cursor { " ❯ " } else { "   " };
                let pointer_style = if is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(confirm_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().bg(confirm_bg)
                };
                // 如果有暂存草稿，显示预览
                let draft = app
                    .ui
                    .tool_ask_drafts
                    .get(app.ui.tool_ask_current_idx)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let label_text = if draft.is_empty() {
                    "✏ 自由输入...".to_string()
                } else {
                    // 截断过长的草稿预览
                    let max_preview = 30;
                    let preview: String = draft.chars().take(max_preview).collect();
                    if draft.chars().count() > max_preview {
                        format!("✏ 已输入: {}...", preview)
                    } else {
                        format!("✏ 已输入: {}", preview)
                    }
                };
                let text_style = if is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(confirm_bg)
                        .add_modifier(Modifier::BOLD)
                } else if draft.is_empty() {
                    Style::default().fg(t.tool_confirm_label).bg(confirm_bg)
                } else {
                    // 有草稿但未选中时，用稍亮的颜色提示
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
            }
        }

        // 底部操作提示
        {
            let inner_w = bubble_max_width.saturating_sub(4);
            lines.push(Line::from(vec![
                Span::styled("  │", Style::default().fg(border_color).bg(confirm_bg)),
                Span::styled(" ".repeat(inner_w), Style::default().bg(confirm_bg)),
                Span::styled("│", Style::default().fg(border_color).bg(confirm_bg)),
            ]));
        }
        let hint = if is_multi {
            " Up/Down Move | Space Toggle | Enter OK | PgUp/PgDn Scroll | Esc Cancel"
        } else {
            " Up/Down Move | Enter OK | PgUp/PgDn Scroll | Esc Cancel"
        };
        lines.push(bordered_line(
            vec![Span::styled(
                hint,
                Style::default().fg(t.text_dim).bg(confirm_bg),
            )],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }
}
