//! 配置字段编辑、Provider 管理、Enter 操作
//!
//! 包含配置字段的编辑逻辑和 Provider 的增删改操作。

use crate::command::chat::app::ChatApp;
use crate::command::chat::app::action::CursorDirection;
use crate::command::chat::app::ui_state::ConfigTab;
use crate::command::chat::storage::ModelProvider;
use crate::constants::CONFIG_FIELDS;

impl ChatApp {
    pub(in crate::command::chat::app::chat_app) fn update_config_enter(&mut self) {
        use crate::command::chat::render::helpers::{
            config_field_raw_value_global, config_field_raw_value_model,
        };
        use crate::constants::CONFIG_GLOBAL_FIELDS_TAB;
        match self.ui.config_tab {
            ConfigTab::Model => {
                if self.state.agent_config.providers.is_empty() {
                    self.show_toast("还没有 Provider，按 a 新增", true);
                    return;
                }
                // 必须在字段层级才能编辑
                if !self.ui.model_in_fields {
                    return;
                }
                // supports_vision 是布尔开关，直接 toggle
                if self.ui.model_field_idx < CONFIG_FIELDS.len()
                    && CONFIG_FIELDS[self.ui.model_field_idx] == "supports_vision"
                    && let Some(p) = self
                        .state
                        .agent_config
                        .providers
                        .get_mut(self.ui.config_provider_idx)
                {
                    p.supports_vision = !p.supports_vision;
                    let status = if p.supports_vision {
                        "开启"
                    } else {
                        "关闭"
                    };
                    self.show_toast(format!("当前 Provider 支持视觉已{}", status), false);
                    return;
                }
                self.ui.config_edit_buf =
                    config_field_raw_value_model(self, self.ui.model_field_idx);
                self.ui.config_edit_cursor = self.ui.config_edit_buf.chars().count();
                self.ui.config_editing = true;
            }
            ConfigTab::Global => {
                let idx = self.ui.config_field_idx;
                if idx < CONFIG_GLOBAL_FIELDS_TAB.len() {
                    let field = CONFIG_GLOBAL_FIELDS_TAB[idx];
                    if field == "auto_restore_session" {
                        self.state.agent_config.auto_restore_session =
                            !self.state.agent_config.auto_restore_session;
                        let status = if self.state.agent_config.auto_restore_session {
                            "开启"
                        } else {
                            "关闭"
                        };
                        self.show_toast(format!("自动恢复会话已{}", status), false);
                        return;
                    }
                    if field == "compact_enabled" {
                        self.state.agent_config.compact.enabled =
                            !self.state.agent_config.compact.enabled;
                        let status = if self.state.agent_config.compact.enabled {
                            "开启"
                        } else {
                            "关闭"
                        };
                        self.show_toast(format!("上下文压缩已{}", status), false);
                        return;
                    }
                    if field == "theme" {
                        self.switch_theme();
                        return;
                    }
                    if field == "flat_bubble" {
                        self.state.agent_config.flat_bubble = !self.state.agent_config.flat_bubble;
                        let status = if self.state.agent_config.flat_bubble {
                            "开启"
                        } else {
                            "关闭"
                        };
                        self.show_toast(format!("扁平气泡已{}", status), false);
                        self.ui.msg_lines_cache = None;
                        return;
                    }
                    if field == "welcome_quote" {
                        self.state.agent_config.welcome_quote =
                            !self.state.agent_config.welcome_quote;
                        let status = if self.state.agent_config.welcome_quote {
                            "开启"
                        } else {
                            "关闭"
                        };
                        self.show_toast(format!("欢迎诗句已{}", status), false);
                        return;
                    }
                    if field == "thinking_style" {
                        let next = self.state.agent_config.thinking_style.next();
                        self.state.agent_config.thinking_style = next;
                        self.show_toast(format!("思考动画: {}", next.display_name()), false);
                        return;
                    }
                    if field == "system_prompt" {
                        self.ui.pending_system_prompt_edit = true;
                        return;
                    }
                    if field == "agent_md" {
                        self.ui.pending_agent_md_edit = true;
                        return;
                    }
                    if field == "compact_exempt_tools" {
                        self.ui.compact_exempt_sublist = true;
                        self.ui.compact_exempt_idx = 0;
                        self.ui.config_scroll_offset = 0;
                        return;
                    }
                    if field == "style" {
                        self.ui.pending_style_edit = true;
                        return;
                    }
                    self.ui.config_edit_buf = config_field_raw_value_global(self, idx);
                    self.ui.config_edit_cursor = self.ui.config_edit_buf.chars().count();
                    self.ui.config_editing = true;
                }
            }
            // Toggle 开关类 Tab
            ConfigTab::Tools | ConfigTab::Skills | ConfigTab::Commands => {
                self.update_toggle_menu_toggle();
            }
            // 这些 Tab 的 Enter 键无特殊操作
            ConfigTab::Session | ConfigTab::Hooks | ConfigTab::Teammates | ConfigTab::Archive => {}
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_config_edit_char(&mut self, c: char) {
        let byte_idx = self
            .ui
            .config_edit_buf
            .char_indices()
            .nth(self.ui.config_edit_cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.ui.config_edit_buf.len());
        self.ui.config_edit_buf.insert(byte_idx, c);
        self.ui.config_edit_cursor += 1;
    }

    pub(in crate::command::chat::app::chat_app) fn update_config_edit_delete(&mut self) {
        if self.ui.config_edit_cursor > 0 {
            let idx = self
                .ui
                .config_edit_buf
                .char_indices()
                .nth(self.ui.config_edit_cursor - 1)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let end_idx = self
                .ui
                .config_edit_buf
                .char_indices()
                .nth(self.ui.config_edit_cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.ui.config_edit_buf.len());
            self.ui.config_edit_buf = format!(
                "{}{}",
                &self.ui.config_edit_buf[..idx],
                &self.ui.config_edit_buf[end_idx..]
            );
            self.ui.config_edit_cursor -= 1;
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_config_edit_delete_forward(&mut self) {
        let char_count = self.ui.config_edit_buf.chars().count();
        if self.ui.config_edit_cursor < char_count {
            let idx = self
                .ui
                .config_edit_buf
                .char_indices()
                .nth(self.ui.config_edit_cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.ui.config_edit_buf.len());
            let end_idx = self
                .ui
                .config_edit_buf
                .char_indices()
                .nth(self.ui.config_edit_cursor + 1)
                .map(|(i, _)| i)
                .unwrap_or(self.ui.config_edit_buf.len());
            self.ui.config_edit_buf = format!(
                "{}{}",
                &self.ui.config_edit_buf[..idx],
                &self.ui.config_edit_buf[end_idx..]
            );
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_config_edit_move_cursor(
        &mut self,
        dir: CursorDirection,
    ) {
        match dir {
            CursorDirection::Up => {
                self.ui.config_edit_cursor = self.ui.config_edit_cursor.saturating_sub(1);
            }
            CursorDirection::Down => {
                let char_count = self.ui.config_edit_buf.chars().count();
                if self.ui.config_edit_cursor < char_count {
                    self.ui.config_edit_cursor += 1;
                }
            }
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_config_edit_clear_line(&mut self) {
        self.ui.config_edit_buf.clear();
        self.ui.config_edit_cursor = 0;
    }

    pub(in crate::command::chat::app::chat_app) fn update_config_edit_submit(&mut self) {
        use crate::command::chat::render::helpers::{
            config_field_set_global, config_field_set_model,
        };
        let val = self.ui.config_edit_buf.clone();
        match self.ui.config_tab {
            ConfigTab::Model => {
                config_field_set_model(self, self.ui.model_field_idx, &val);
            }
            ConfigTab::Global => {
                config_field_set_global(self, self.ui.config_field_idx, &val);
            }
            // 这些 Tab 不支持字段编辑提交
            ConfigTab::Session
            | ConfigTab::Tools
            | ConfigTab::Skills
            | ConfigTab::Hooks
            | ConfigTab::Commands
            | ConfigTab::Teammates
            | ConfigTab::Archive => {}
        }
        self.ui.config_editing = false;
    }

    pub(in crate::command::chat::app::chat_app) fn update_config_add_provider(&mut self) {
        let new_provider = ModelProvider {
            name: format!("Provider-{}", self.state.agent_config.providers.len() + 1),
            api_base: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: String::new(),
            supports_vision: false,
        };
        self.state.agent_config.providers.push(new_provider);
        self.ui.config_provider_idx = self.state.agent_config.providers.len() - 1;
        self.ui.config_field_idx = 0;
        self.show_toast("已新增 Provider，请填写配置", false);
    }

    pub(in crate::command::chat::app::chat_app) fn update_config_delete_provider(&mut self) {
        let count = self.state.agent_config.providers.len();
        if count == 0 {
            self.show_toast("没有可删除的 Provider", true);
        } else {
            let removed_name = self.state.agent_config.providers[self.ui.config_provider_idx]
                .name
                .clone();
            self.state
                .agent_config
                .providers
                .remove(self.ui.config_provider_idx);
            if self.ui.config_provider_idx >= self.state.agent_config.providers.len()
                && self.ui.config_provider_idx > 0
            {
                self.ui.config_provider_idx -= 1;
            }
            if self.state.agent_config.active_index >= self.state.agent_config.providers.len()
                && self.state.agent_config.active_index > 0
            {
                self.state.agent_config.active_index -= 1;
            }
            self.show_toast(format!("已删除 Provider: {}", removed_name), false);
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_config_set_active_provider(&mut self) {
        if !self.state.agent_config.providers.is_empty() {
            self.state.agent_config.active_index = self.ui.config_provider_idx;
            let name = self.state.agent_config.providers[self.ui.config_provider_idx]
                .name
                .clone();
            self.show_toast(format!("已设为活跃模型: {}", name), false);
        }
    }
}
