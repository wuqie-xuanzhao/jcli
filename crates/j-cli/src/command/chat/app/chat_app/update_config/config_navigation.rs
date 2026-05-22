//! 配置界面导航、Tab 切换、字段选择
//!
//! 包含配置界面的导航逻辑，如 Tab 切换、字段选择、层级切换等。

use crate::command::chat::app::ChatApp;
use crate::command::chat::app::action::CursorDirection;
use crate::command::chat::app::ui_state::ConfigTab;
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS_TAB};

/// 所有字段数 = provider 字段 + 全局字段
/// 根据当前 tab 计算字段总数
pub(in crate::command::chat::app::chat_app) fn config_tab_field_count(app: &ChatApp) -> usize {
    match app.ui.config_tab {
        ConfigTab::Model => CONFIG_FIELDS.len(),
        ConfigTab::Global => CONFIG_GLOBAL_FIELDS_TAB.len(),
        ConfigTab::Tools => app.tool_registry.tool_names().len(),
        ConfigTab::Skills => app.state.loaded_skills.len(),
        ConfigTab::Commands => app.state.loaded_commands.len(),
        ConfigTab::Hooks => app
            .hook_manager
            .lock()
            .map(|m| m.list_hooks().len())
            .unwrap_or(0),
        ConfigTab::Session => app.ui.session_list.len(),
        ConfigTab::Teammates => app
            .teammate_manager
            .lock()
            .map(|m| m.teammates.len())
            .unwrap_or(0),
        ConfigTab::Archive => app.ui.archives.len(),
    }
}

impl ChatApp {
    pub(in crate::command::chat::app::chat_app) fn update_config_navigate(
        &mut self,
        dir: CursorDirection,
    ) {
        // Model tab 使用层级导航
        if self.ui.config_tab == ConfigTab::Model {
            if self.ui.model_in_fields {
                // 字段层级：上下切换配置字段
                let total = CONFIG_FIELDS.len();
                if total == 0 {
                    return;
                }
                match dir {
                    CursorDirection::Up => {
                        if self.ui.model_field_idx > 0 {
                            self.ui.model_field_idx -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        if self.ui.model_field_idx < total - 1 {
                            self.ui.model_field_idx += 1;
                        }
                    }
                }
            } else {
                // Provider 列表层级：上下切换 Provider
                let count = self.state.agent_config.providers.len();
                if count == 0 {
                    return;
                }
                match dir {
                    CursorDirection::Up => {
                        if self.ui.config_provider_idx == 0 {
                            self.ui.config_provider_idx = count - 1;
                        } else {
                            self.ui.config_provider_idx -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        self.ui.config_provider_idx = (self.ui.config_provider_idx + 1) % count;
                    }
                }
            }
            return;
        }

        // 其他 Tab 保持原有逻辑
        let total_fields = config_tab_field_count(self);
        if total_fields == 0 {
            return;
        }
        match dir {
            CursorDirection::Up => {
                if self.ui.config_field_idx > 0 {
                    self.ui.config_field_idx -= 1;
                }
            }
            CursorDirection::Down => {
                if self.ui.config_field_idx < total_fields - 1 {
                    self.ui.config_field_idx += 1;
                }
            }
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_config_switch_tab(
        &mut self,
        dir: CursorDirection,
    ) {
        use crate::command::chat::infra::archive;
        self.ui.config_tab = match dir {
            CursorDirection::Down => self.ui.config_tab.next(),
            CursorDirection::Up => self.ui.config_tab.prev(),
        };
        self.ui.config_field_idx = 0;
        self.ui.config_scroll_offset = 0;
        self.ui.config_editing = false;
        // 切换到 Session tab 时自动加载列表
        if self.ui.config_tab == ConfigTab::Session {
            self.update_load_session_list();
        }
        // 切换到 Archive tab 时自动加载归档列表
        if self.ui.config_tab == ConfigTab::Archive {
            self.ui.archives = archive::list_archives();
            self.ui.archive_list_index = 0;
            self.ui.restore_confirm_needed = false;
        }
    }

    /// 配置界面：鼠标直接点击跳转到指定 Tab
    pub(in crate::command::chat::app::chat_app) fn update_config_switch_tab_to(
        &mut self,
        tab: ConfigTab,
    ) {
        use crate::command::chat::infra::archive;
        self.ui.config_tab = tab;
        self.ui.config_field_idx = 0;
        self.ui.config_scroll_offset = 0;
        self.ui.config_editing = false;
        if self.ui.config_tab == ConfigTab::Session {
            self.update_load_session_list();
        }
        if self.ui.config_tab == ConfigTab::Archive {
            self.ui.archives = archive::list_archives();
            self.ui.archive_list_index = 0;
            self.ui.restore_confirm_needed = false;
        }
    }

    /// 配置界面：鼠标点击选中指定字段索引
    pub(in crate::command::chat::app::chat_app) fn update_config_field_select(
        &mut self,
        idx: usize,
    ) {
        let total = config_tab_field_count(self);
        if total > 0 && idx < total {
            self.ui.config_field_idx = idx;
        }
    }

    /// 配置界面：鼠标点击 Model tab 左侧 Provider 列表选中指定 Provider
    pub(in crate::command::chat::app::chat_app) fn update_config_provider_select(
        &mut self,
        idx: usize,
    ) {
        let count = self.state.agent_config.providers.len();
        if idx < count {
            self.ui.config_provider_idx = idx;
            // 切换到 Provider 层级（左侧面板聚焦）
            self.ui.model_in_fields = false;
            // 重置右侧字段索引
            self.ui.model_field_idx = 0;
        }
    }

    /// Tools tab：Tab 键切换层级（工具列表 <-> 选项区）。
    /// 选项区包含 2 个选项：索引 0 = 启用/禁用，索引 1 = defer 开关。
    /// 详见 `config/tools.rs` 中的渲染逻辑。
    pub(in crate::command::chat::app::chat_app) fn update_tools_toggle_level(&mut self) {
        if self.ui.config_tab != ConfigTab::Tools {
            return;
        }
        if self.ui.tools_in_options {
            // 从选项区返回工具列表
            self.ui.tools_in_options = false;
        } else {
            // 进入选中工具的选项区
            self.ui.tools_in_options = true;
            self.ui.tools_option_idx = 0; // 默认焦点在"启用"
        }
    }

    /// Model tab：Tab 键切换层级（Provider 列表 <-> 配置字段）。
    /// 在 Provider 列表层按 Tab 进入右侧配置字段区，
    /// 在字段区按 Tab 返回 Provider 列表。
    pub(in crate::command::chat::app::chat_app) fn update_model_toggle_level(&mut self) {
        if self.ui.config_tab != ConfigTab::Model {
            return;
        }
        if self.ui.config_editing {
            // 编辑模式下 Tab 不切换层级
            return;
        }
        if self.ui.model_in_fields {
            // 从字段区返回 Provider 列表
            self.ui.model_in_fields = false;
        } else if !self.state.agent_config.providers.is_empty() {
            // 进入选中 Provider 的配置字段区
            self.ui.model_in_fields = true;
            self.ui.model_field_idx = 0;
        }
    }
}
