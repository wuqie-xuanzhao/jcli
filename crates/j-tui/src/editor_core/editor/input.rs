//! 输入处理：handle_input 主分发器与 handle_mode_input 模式字符处理

use super::super::history::Snapshot;
use super::super::vim::{
    CmdItem, Input, Key, Mode, Transition, filter_commands, filter_insert_commands,
};
use super::MarkdownEditor;

impl MarkdownEditor {
    /// 处理输入
    #[allow(clippy::too_many_lines)]
    pub fn handle_input(&mut self, input: &Input) -> super::EditorAction {
        // 键盘输入解除滚动锁定
        self.viewport.scroll_locked = false;

        // 清除状态消息
        self.status_message = None;

        // 帮助弹窗模式：拦截所有按键
        if self.vim.mode() == &Mode::HelpPopup {
            return self.handle_help_popup(input);
        }

        // 主题选择模式：拦截所有按键
        if self.vim.mode() == &Mode::ThemeSelect {
            return self.handle_theme_select(input);
        }

        // 全局快捷键
        if input.ctrl && input.key == Key::Char('s') {
            return super::EditorAction::Submit(self.buffer.to_string());
        }
        if input.ctrl && input.key == Key::Char('q') {
            return super::EditorAction::Cancel;
        }

        // 处理撤销
        if self.vim.mode() == &Mode::Normal && input.key == Key::Char('u') && !input.ctrl {
            self.undo();
            return super::EditorAction::Continue;
        }

        // 处理重做
        if self.vim.mode() == &Mode::Normal && input.key == Key::Char('r') && input.ctrl {
            self.redo();
            return super::EditorAction::Continue;
        }

        // 处理搜索跳转
        if self.vim.mode() == &Mode::Normal && self.search.is_searching() {
            if input.key == Key::Char('n') && !input.ctrl {
                self.search_next();
                return super::EditorAction::Continue;
            }
            if input.key == Key::Char('N') && !input.ctrl {
                self.search_prev();
                return super::EditorAction::Continue;
            }
            // Enter 跳到下一个匹配（直观一致）
            if input.key == Key::Enter && !input.ctrl {
                self.search_next();
                return super::EditorAction::Continue;
            }
            // Esc 清除搜索高亮
            if input.key == Key::Esc && !input.ctrl {
                self.search.clear();
                return super::EditorAction::Continue;
            }
        }

        // 命令面板模式：拦截上下键和回车键
        // 先克隆 filter 以释放 self.vim 的借用，避免后续调用 execute_command 时的借用冲突
        //
        // 同时处理两种面板：
        //  - CommandPanel：Normal 模式触发，命令列表 = COMMANDS
        //  - InsertCommandPanel：Insert 模式触发，命令列表 = INSERT_COMMANDS
        {
            #[derive(Clone, Copy)]
            enum PanelKind {
                Normal,
                Insert,
            }
            let panel_state: Option<(PanelKind, String)> = match self.vim.mode() {
                Mode::CommandPanel(f) => Some((PanelKind::Normal, f.clone())),
                Mode::InsertCommandPanel(f) => Some((PanelKind::Insert, f.clone())),
                _ => None,
            };
            if let Some((kind, filter)) = panel_state {
                let filtered: Vec<&CmdItem> = match kind {
                    PanelKind::Normal => filter_commands(&filter),
                    PanelKind::Insert => filter_insert_commands(&filter),
                };
                match input.key {
                    Key::Up => {
                        if !filtered.is_empty() {
                            if self.cmd_popup_selected > 0 {
                                self.cmd_popup_selected -= 1;
                            } else {
                                self.cmd_popup_selected = filtered.len() - 1;
                            }
                        }
                        return super::EditorAction::Continue;
                    }
                    Key::Down => {
                        if !filtered.is_empty() {
                            if self.cmd_popup_selected < filtered.len() - 1 {
                                self.cmd_popup_selected += 1;
                            } else {
                                self.cmd_popup_selected = 0;
                            }
                        }
                        return super::EditorAction::Continue;
                    }
                    Key::Enter => {
                        let selected = self
                            .cmd_popup_selected
                            .min(filtered.len().saturating_sub(1));
                        if let Some(cmd) = filtered.get(selected) {
                            match kind {
                                PanelKind::Normal => {
                                    let full_cmd = if cmd.name == "jump" {
                                        filter
                                    } else {
                                        cmd.name.to_string()
                                    };
                                    return self.execute_command(&full_cmd);
                                }
                                PanelKind::Insert => {
                                    return self.execute_insert_command(cmd.name, &filter);
                                }
                            }
                        }
                        // 没有匹配项：恢复到对应的来源模式
                        match kind {
                            PanelKind::Normal => self.vim.set_mode(Mode::Normal),
                            PanelKind::Insert => {
                                self.vim.set_mode(Mode::Insert);
                                self.insert_panel_anchor = None;
                            }
                        }
                        return super::EditorAction::Continue;
                    }
                    Key::Esc => {
                        // Insert 面板：保留已插入的 / 与 filter 文本，回到 Insert
                        // Normal 面板：保持原有行为，由 vim 状态机处理（fallthrough）
                        if matches!(kind, PanelKind::Insert) {
                            self.vim.set_mode(Mode::Insert);
                            self.insert_panel_anchor = None;
                            return super::EditorAction::Continue;
                        }
                    }
                    _ => {} // 其他按键交由后续 handle_mode_input 处理
                }
            }
        }

        // 折行感知的上下移动
        // j/k 只在 Normal 模式拦截，方向键在所有模式拦截
        if self.wrap.is_enabled() {
            let is_normal = self.vim.mode() == &Mode::Normal;
            let is_down = matches!(input.key, Key::Down)
                || (is_normal && matches!(input.key, Key::Char('j')));
            let is_up =
                matches!(input.key, Key::Up) || (is_normal && matches!(input.key, Key::Char('k')));

            if is_down && !input.ctrl {
                self.move_cursor_visual_down();
                return super::EditorAction::Continue;
            }
            if is_up && !input.ctrl {
                self.move_cursor_visual_up();
                return super::EditorAction::Continue;
            }
        }

        // Vim 状态机处理
        let old_mode = self.vim.mode().clone();
        let transition = self.vim.handle_input(input, &mut self.buffer);

        match transition {
            Transition::Mode(new_mode) => {
                // 如果从 Insert 模式退出，保存 undo 点
                if old_mode == Mode::Insert && new_mode != Mode::Insert {
                    self.vim
                        .push_snapshot(Snapshot::new(self.buffer.snapshot()), self.buffer.cursor());
                }
                // 从 Search 模式退出时跳转到当前匹配结果
                if matches!(old_mode, Mode::Search(_))
                    && new_mode == Mode::Normal
                    && let Some(m) = self.search.current_match()
                {
                    self.buffer.set_cursor(m.line, m.start);
                }
                if matches!(old_mode, Mode::Search(_)) {
                    self.cursor_before_search = None;
                }
                // 进入 Insert 命令面板时记录触发的 `/` 字符位置（光标已前进 1 位）
                if old_mode == Mode::Insert && matches!(new_mode, Mode::InsertCommandPanel(_)) {
                    let (row, col) = self.buffer.cursor();
                    self.insert_panel_anchor = Some((row, col.saturating_sub(1)));
                }
                self.vim.set_mode(new_mode);
                self.rebuild_wrap_cache();
            }
            Transition::Submit => {
                return super::EditorAction::Submit(self.buffer.to_string());
            }
            Transition::Cancel => {
                return super::EditorAction::Cancel;
            }
            Transition::SearchAbort => {
                // Esc 取消搜索：恢复光标到搜索前位置，清除搜索高亮
                if let Some(pos) = self.cursor_before_search.take() {
                    self.buffer.set_cursor(pos.0, pos.1);
                }
                self.search.clear();
                self.vim.set_mode(Mode::Normal);
            }
            Transition::Nop => {
                // 处理 Command/Search 模式的字符输入
                self.handle_mode_input(input);
            }
            Transition::NeedRebuild => {
                // Normal 模式下的破坏性操作（dd/x/dw/d$）需要 undo 点
                if old_mode == Mode::Normal {
                    self.vim
                        .push_snapshot(Snapshot::new(self.buffer.snapshot()), self.buffer.cursor());
                }
                self.rebuild_wrap_cache();
            }
            Transition::ToggleWrap(enabled) => {
                self.wrap.set_enabled(enabled);
                self.rebuild_wrap_cache();
            }
            Transition::ExecuteCommand(cmd) => {
                return self.execute_command(&cmd);
            }
            Transition::ClipboardCopy => {
                if let Some(text) = self.vim.get_selection_text(&self.buffer) {
                    self.vim.set_yank_register(&text);
                    let _ = self.copy_to_clipboard(&text);
                }
                self.vim.set_mode(Mode::Normal);
                self.rebuild_wrap_cache();
            }
            Transition::Save => {
                return super::EditorAction::Save(self.buffer.to_string());
            }
        }

        super::EditorAction::Continue
    }

    /// 处理模式特定的输入
    #[allow(clippy::too_many_lines)]
    fn handle_mode_input(&mut self, input: &Input) {
        match self.vim.mode() {
            Mode::Command(cmd) => {
                let mut cmd = cmd.clone();
                match &input.key {
                    Key::Char(c) => cmd.push(*c),
                    Key::Backspace => {
                        cmd.pop();
                    }
                    _ => {} // 忽略其他按键（如功能键、组合键等）
                }
                self.vim.set_mode(Mode::Command(cmd));
            }
            Mode::Search(pattern) => {
                let mut pattern = pattern.clone();
                match &input.key {
                    Key::Char(c)
                        // 过滤控制字符（如 Esc 产生的 \x1b）
                        if !c.is_control() => {
                        pattern.push(*c);
                        self.search.search(&pattern, self.buffer.lines());
                    }
                    Key::Backspace => {
                        pattern.pop();
                        self.search.search(&pattern, self.buffer.lines());
                    }
                    _ => {} // 忽略其他按键（如功能键、组合键等）
                }
                self.vim.set_mode(Mode::Search(pattern));
            }
            Mode::CommandPanel(filter) => {
                let mut filter = filter.clone();
                match &input.key {
                    Key::Char(c) => {
                        filter.push(*c);
                        self.cmd_popup_selected = 0;
                    }
                    Key::Backspace => {
                        if !filter.is_empty() {
                            filter.pop();
                            self.cmd_popup_selected = 0;
                        } else {
                            self.vim.set_mode(Mode::Normal);
                            return;
                        }
                    }
                    _ => {} // 忽略其他按键（如功能键、组合键等）
                }
                self.vim.set_mode(Mode::CommandPanel(filter));
            }
            Mode::InsertCommandPanel(filter) => {
                // Insert 模式专用面板：触发 `/` 已经写入 buffer。
                // 这里继续把字符同步插入 buffer + filter，让用户的真实输入和面板状态一致。
                let mut filter = filter.clone();
                match &input.key {
                    Key::Char(c) => {
                        // `/` 字符已经走 vim::handle_insert_mode 的分支被插入；
                        // 这里只处理后续字符。
                        self.buffer.insert_char(*c);
                        filter.push(*c);
                        self.cmd_popup_selected = 0;

                        // 如果新输入后没有任何匹配项，自动关闭面板回到 Insert
                        // （让用户能够正常打 `https://` 之类的真实文本）
                        if filter_insert_commands(&filter).is_empty() {
                            self.vim.set_mode(Mode::Insert);
                            self.insert_panel_anchor = None;
                            self.rebuild_wrap_cache();
                            return;
                        }
                        self.rebuild_wrap_cache();
                    }
                    Key::Backspace => {
                        if !filter.is_empty() {
                            // 同步从 buffer 删除最后一个 filter 字符
                            self.buffer.backspace();
                            filter.pop();
                            self.cmd_popup_selected = 0;
                            self.rebuild_wrap_cache();
                        } else {
                            // filter 为空 → 删除触发的 `/`，回到 Insert
                            self.buffer.backspace();
                            self.vim.set_mode(Mode::Insert);
                            self.insert_panel_anchor = None;
                            self.rebuild_wrap_cache();
                            return;
                        }
                    }
                    _ => {}
                }
                self.vim.set_mode(Mode::InsertCommandPanel(filter));
            }
            _ => {}
        }
    }
}
