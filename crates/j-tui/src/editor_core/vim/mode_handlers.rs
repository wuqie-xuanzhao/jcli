//! Vim 模式处理函数
//!
//! 从 `Vim` impl 块中提取的各模式输入处理方法。

use super::{Input, Key, Mode, TextBuffer, Transition};
use super::commands::filter_commands;

impl super::Vim {
    pub(super) fn handle_insert_mode(
        &mut self,
        input: &Input,
        buffer: &mut TextBuffer,
    ) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Enter => {
                buffer.insert_newline();
                Transition::NeedRebuild
            }
            Key::Backspace => {
                buffer.backspace();
                Transition::NeedRebuild
            }
            Key::Delete => {
                buffer.delete_char();
                Transition::NeedRebuild
            }
            Key::Left => {
                buffer.move_cursor_back();
                Transition::Nop
            }
            Key::Right => {
                buffer.move_cursor_forward();
                Transition::Nop
            }
            Key::Up => {
                buffer.move_cursor_up();
                Transition::Nop
            }
            Key::Down => {
                buffer.move_cursor_down();
                Transition::Nop
            }
            Key::Char(c) => {
                buffer.insert_char(c);
                // 输入 `/` 时弹出 Insert 命令面板（image / 等）。
                // `/` 字符已经被正常插入到 buffer——面板只是叠加层，由 editor.rs 处理后续输入。
                if c == '/' {
                    Transition::Mode(Mode::InsertCommandPanel(String::new()))
                } else {
                    Transition::NeedRebuild
                }
            }
            Key::Tab => {
                buffer.insert_str("    ");
                Transition::NeedRebuild
            }
            _ => Transition::Nop,
        }
    }

    pub(super) fn handle_command_mode(&mut self, input: &Input, cmd: String) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Enter => {
                let trimmed = cmd.trim();
                match trimmed {
                    "w" => Transition::Save,
                    "wq" | "x" => Transition::Submit,
                    "q" | "q!" => Transition::Cancel,
                    "set wrap" => Transition::ToggleWrap(true),
                    "set nowrap" => Transition::ToggleWrap(false),
                    _ => Transition::Mode(Mode::Normal),
                }
            }
            _ => Transition::Nop,
        }
    }

    pub(super) fn handle_search_mode(&mut self, input: &Input, _pattern: String) -> Transition {
        match input.key {
            Key::Esc => Transition::SearchAbort,
            Key::Enter => Transition::Mode(Mode::Normal),
            _ => Transition::Nop,
        }
    }

    pub(super) fn handle_command_panel_mode(
        &mut self,
        input: &Input,
        filter: String,
    ) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Enter => {
                // 查找匹配的命令
                let matched = filter_commands(&filter);
                if let Some(cmd) = matched.first() {
                    let cmd_name = cmd.name.to_string();
                    // 需要带参数的命令（如 jump）保留 filter 中的参数部分
                    let full_cmd = if cmd_name == "jump" {
                        // filter 可能是 "jump 10" 或 "ju 10"
                        filter.clone()
                    } else {
                        cmd_name
                    };
                    Transition::ExecuteCommand(full_cmd)
                } else {
                    Transition::Mode(Mode::Normal)
                }
            }
            _ => Transition::Nop,
        }
    }

    pub(super) fn handle_visual_mode(
        &mut self,
        input: &Input,
        buffer: &mut TextBuffer,
    ) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Char('y') => {
                if let Some(text) = self.get_selection_text(buffer) {
                    self.yank_register = text;
                }
                Transition::Mode(Mode::Normal)
            }
            // 复制选区到系统剪贴板
            Key::Char('c') => Transition::ClipboardCopy,
            Key::Char('h') | Key::Left => {
                buffer.move_cursor_back();
                Transition::Nop
            }
            Key::Char('j') | Key::Down => {
                buffer.move_cursor_down();
                Transition::Nop
            }
            Key::Char('k') | Key::Up => {
                buffer.move_cursor_up();
                Transition::Nop
            }
            Key::Char('l') | Key::Right => {
                buffer.move_cursor_forward();
                Transition::Nop
            }
            _ => Transition::Nop,
        }
    }

    pub(super) fn handle_operator_mode(
        &mut self,
        input: &Input,
        op: char,
        buffer: &mut TextBuffer,
    ) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Char('d') if op == 'd' => {
                let (row, _) = buffer.cursor();
                self.yank_register = buffer.line(row).cloned().unwrap_or_default();
                buffer.delete_line();
                Transition::NeedRebuild
            }
            Key::Char('w') => match op {
                'd' => {
                    buffer.delete_word();
                    Transition::NeedRebuild
                }
                'c' => {
                    buffer.delete_word();
                    Transition::Mode(Mode::Insert)
                }
                _ => Transition::Mode(Mode::Normal),
            },
            Key::Char('$') => match op {
                'd' => {
                    let (row, col) = buffer.cursor();
                    if let Some(line) = buffer.line(row) {
                        let chars: Vec<char> = line.chars().collect();
                        self.yank_register = chars[col..].iter().collect();
                    }
                    buffer.delete_line_by_end();
                    Transition::NeedRebuild
                }
                'c' => {
                    let (row, col) = buffer.cursor();
                    if let Some(line) = buffer.line(row) {
                        let chars: Vec<char> = line.chars().collect();
                        self.yank_register = chars[col..].iter().collect();
                    }
                    buffer.delete_line_by_end();
                    Transition::Mode(Mode::Insert)
                }
                _ => Transition::Mode(Mode::Normal),
            },
            Key::Char('c') if op == 'c' => {
                let (row, _) = buffer.cursor();
                self.yank_register = buffer.line(row).cloned().unwrap_or_default();
                buffer.delete_line_by_end();
                buffer.move_cursor_head();
                Transition::Mode(Mode::Insert)
            }
            _ => Transition::Mode(Mode::Normal),
        }
    }
}
