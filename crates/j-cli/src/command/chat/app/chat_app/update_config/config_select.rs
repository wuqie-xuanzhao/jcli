//! 各种列表选择操作
//!
//! 包含模型选择、主题选择、Teammates 导航和命令创建等选择操作。

use crate::command::chat::app::ChatApp;
use crate::command::chat::app::action::CursorDirection;

// ========== 模型选择 ==========

impl ChatApp {
    pub(in crate::command::chat::app::chat_app) fn update_model_select_navigate(
        &mut self,
        dir: CursorDirection,
    ) {
        let count = self.state.agent_config.providers.len();
        if count > 0 {
            match dir {
                CursorDirection::Up => {
                    let i = self
                        .ui
                        .model_list_state
                        .selected()
                        .map(|i| if i == 0 { count - 1 } else { i - 1 })
                        .unwrap_or(0);
                    self.ui.model_list_state.select(Some(i));
                }
                CursorDirection::Down => {
                    let i = self
                        .ui
                        .model_list_state
                        .selected()
                        .map(|i| if i >= count - 1 { 0 } else { i + 1 })
                        .unwrap_or(0);
                    self.ui.model_list_state.select(Some(i));
                }
            }
        }
    }

    // ========== 主题选择 ==========

    pub(in crate::command::chat::app::chat_app) fn update_theme_select_navigate(
        &mut self,
        dir: CursorDirection,
    ) {
        let count = crate::theme::ThemeName::all().len();
        if count > 0 {
            match dir {
                CursorDirection::Up => {
                    let i = self
                        .ui
                        .theme_list_state
                        .selected()
                        .map(|i| if i == 0 { count - 1 } else { i - 1 })
                        .unwrap_or(0);
                    self.ui.theme_list_state.select(Some(i));
                }
                CursorDirection::Down => {
                    let i = self
                        .ui
                        .theme_list_state
                        .selected()
                        .map(|i| if i >= count - 1 { 0 } else { i + 1 })
                        .unwrap_or(0);
                    self.ui.theme_list_state.select(Some(i));
                }
            }
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_theme_select_confirm(&mut self) {
        use crate::command::chat::storage::save_agent_config;
        use crate::theme::{Theme, ThemeName};
        if let Some(sel) = self.ui.theme_list_state.selected() {
            let all = ThemeName::all();
            if sel < all.len() {
                self.state.agent_config.theme = all[sel].clone();
                self.ui.theme = Theme::from_name(&all[sel]);
                self.ui.msg_lines_cache = None;
                let _ = save_agent_config(&self.state.agent_config);
                let name = all[sel].display_name();
                self.show_toast(format!("已切换主题: {}", name), false);
            }
        }
        self.ui.mode = crate::command::chat::app::ui_state::ChatMode::Chat;
    }

    /// Teammates Tab：导航上下移动选中指针
    pub(in crate::command::chat::app::chat_app) fn update_teammates_navigate(
        &mut self,
        dir: CursorDirection,
    ) {
        let count = self
            .teammate_manager
            .lock()
            .map(|m| m.teammates.len())
            .unwrap_or(0);
        if count == 0 {
            return;
        }
        match dir {
            CursorDirection::Up => {
                if self.ui.teammate_list_index > 0 {
                    self.ui.teammate_list_index -= 1;
                }
            }
            CursorDirection::Down => {
                if self.ui.teammate_list_index < count - 1 {
                    self.ui.teammate_list_index += 1;
                }
            }
        }
    }

    /// Teammates Tab：鼠标点击选中指定索引
    pub(in crate::command::chat::app::chat_app) fn update_teammates_select(&mut self, idx: usize) {
        let count = self
            .teammate_manager
            .lock()
            .map(|m| m.teammates.len())
            .unwrap_or(0);
        if count > 0 && idx < count {
            self.ui.teammate_list_index = idx;
        }
    }

    // ========== Commands 创建 ==========

    /// 进入选择命令保存级别模式
    pub(in crate::command::chat::app::chat_app) fn update_config_command_select_source(&mut self) {
        use crate::command::chat::app::ui_state::CommandsMode;
        use crate::command::chat::infra::command::CommandSource;

        if crate::command::chat::infra::command::project_commands_dir().is_some() {
            self.ui.commands_mode = CommandsMode::SelectSource;
            self.ui.commands_source_idx = 0;
        } else {
            // 没有项目级目录，直接使用用户级
            self.ui.command_create_source = CommandSource::User;
            self.ui.pending_command_create = true;
        }
    }

    /// 选择命令保存级别导航
    pub(in crate::command::chat::app::chat_app) fn update_config_command_navigate_source(
        &mut self,
        dir: CursorDirection,
    ) {
        match dir {
            CursorDirection::Up => {
                if self.ui.commands_source_idx > 0 {
                    self.ui.commands_source_idx -= 1;
                }
            }
            CursorDirection::Down => {
                if self.ui.commands_source_idx < 1 {
                    self.ui.commands_source_idx += 1;
                }
            }
        }
    }

    /// 确认命令保存级别选择
    pub(in crate::command::chat::app::chat_app) fn update_config_command_confirm_source(&mut self) {
        use crate::command::chat::app::ui_state::CommandsMode;
        use crate::command::chat::infra::command::CommandSource;

        self.ui.command_create_source = if self.ui.commands_source_idx == 0 {
            CommandSource::User
        } else {
            CommandSource::Project
        };
        self.ui.commands_mode = CommandsMode::Normal;
        self.ui.pending_command_create = true;
    }

    /// 取消命令创建
    pub(in crate::command::chat::app::chat_app) fn update_config_command_cancel(&mut self) {
        use crate::command::chat::app::ui_state::CommandsMode;
        self.ui.commands_mode = CommandsMode::Normal;
    }
}
