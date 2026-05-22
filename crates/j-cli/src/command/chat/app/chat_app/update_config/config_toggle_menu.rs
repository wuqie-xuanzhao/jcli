//! Tools/Skills/Commands/Hooks 启用禁用管理
//!
//! 包含各种开关菜单的导航、切换、批量启用/禁用等操作。

use super::config_navigation::config_tab_field_count;
use crate::command::chat::app::ChatApp;
use crate::command::chat::app::action::CursorDirection;
use crate::command::chat::app::ui_state::ConfigTab;

impl ChatApp {
    pub(in crate::command::chat::app::chat_app) fn update_toggle_menu_navigate(
        &mut self,
        dir: CursorDirection,
    ) {
        // Tools tab 使用层级导航
        if self.ui.config_tab == ConfigTab::Tools {
            if self.ui.tools_in_options {
                // 选项层级：上下切换启用/defer
                match dir {
                    CursorDirection::Up => {
                        if self.ui.tools_option_idx > 0 {
                            self.ui.tools_option_idx -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        if self.ui.tools_option_idx < 1 {
                            self.ui.tools_option_idx += 1;
                        }
                    }
                }
            } else {
                // 工具列表层级：上下切换工具
                let total = self.tool_registry.tool_names().len();
                if total == 0 {
                    return;
                }
                match dir {
                    CursorDirection::Up => {
                        if self.ui.config_field_idx == 0 {
                            self.ui.config_field_idx = total - 1;
                        } else {
                            self.ui.config_field_idx -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        self.ui.config_field_idx = (self.ui.config_field_idx + 1) % total;
                    }
                }
            }
            return;
        }

        // 其他 Tab 保持原有逻辑
        let total = config_tab_field_count(self);
        if total == 0 {
            return;
        }
        match dir {
            CursorDirection::Up => {
                if self.ui.config_field_idx == 0 {
                    self.ui.config_field_idx = total - 1;
                } else {
                    self.ui.config_field_idx -= 1;
                }
            }
            CursorDirection::Down => {
                self.ui.config_field_idx = (self.ui.config_field_idx + 1) % total;
            }
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_toggle_menu_toggle(&mut self) {
        if self.ui.config_tab == ConfigTab::Tools {
            // 根据当前层级和焦点决定 toggle 哪个选项
            if self.ui.tools_in_options {
                let tool_names = self.tool_registry.tool_names();
                if let Some(name) = tool_names.get(self.ui.config_field_idx) {
                    let name = name.to_string();
                    if self.ui.tools_option_idx == 0 {
                        // toggle 启用状态
                        if let Some(pos) = self
                            .state
                            .agent_config
                            .disabled_tools
                            .iter()
                            .position(|d| d == &name)
                        {
                            self.state.agent_config.disabled_tools.remove(pos);
                        } else {
                            self.state.agent_config.disabled_tools.push(name);
                        }
                    } else {
                        // toggle defer 状态（仅对启用的工具有效）
                        let is_enabled = !self
                            .state
                            .agent_config
                            .disabled_tools
                            .iter()
                            .any(|d| d == &name);
                        if is_enabled {
                            // 双写：同时修改 agent_config（持久化）和 deferred_tools（运行时）
                            let mut deferred = match self.deferred_tools.lock() {
                                Ok(guard) => guard,
                                Err(e) => e.into_inner(),
                            };
                            if let Some(pos) = deferred.iter().position(|d| d == &name) {
                                deferred.remove(pos);
                                // 同步移除 agent_config 中的对应项
                                if let Some(pos2) = self
                                    .state
                                    .agent_config
                                    .deferred_tools
                                    .iter()
                                    .position(|d| d == &name)
                                {
                                    self.state.agent_config.deferred_tools.remove(pos2);
                                }
                                // 同步移除 session_loaded_deferred 中的记录
                                if let Ok(mut loaded) = self.session_loaded_deferred.lock() {
                                    loaded.retain(|n| n != &name);
                                }
                            } else {
                                deferred.push(name.clone());
                                self.state.agent_config.deferred_tools.push(name);
                            }
                        }
                    }
                }
            } else {
                // 工具列表层级，Enter 也 toggle 启用状态
                let tool_names = self.tool_registry.tool_names();
                if let Some(name) = tool_names.get(self.ui.config_field_idx) {
                    let name = name.to_string();
                    if let Some(pos) = self
                        .state
                        .agent_config
                        .disabled_tools
                        .iter()
                        .position(|d| d == &name)
                    {
                        self.state.agent_config.disabled_tools.remove(pos);
                    } else {
                        self.state.agent_config.disabled_tools.push(name);
                    }
                }
            }
            return;
        }

        // 其他 Tab 保持原有逻辑
        if self.ui.config_tab == ConfigTab::Skills
            && let Some(skill) = self.state.loaded_skills.get(self.ui.config_field_idx)
        {
            let name = skill.frontmatter.name.clone();
            if let Some(pos) = self
                .state
                .agent_config
                .disabled_skills
                .iter()
                .position(|d| d == &name)
            {
                self.state.agent_config.disabled_skills.remove(pos);
            } else {
                self.state.agent_config.disabled_skills.push(name);
            }
        } else if self.ui.config_tab == ConfigTab::Commands
            && let Some(cmd) = self.state.loaded_commands.get(self.ui.config_field_idx)
        {
            let name = cmd.frontmatter.name.clone();
            if let Some(pos) = self
                .state
                .agent_config
                .disabled_commands
                .iter()
                .position(|d| d == &name)
            {
                self.state.agent_config.disabled_commands.remove(pos);
            } else {
                self.state.agent_config.disabled_commands.push(name);
            }
        } else if self.ui.config_tab == ConfigTab::Hooks
            && let Ok(manager) = self.hook_manager.lock()
        {
            let hooks = manager.list_hooks();
            if let Some(entry) = hooks.get(self.ui.config_field_idx) {
                let uid = entry.unique_id.clone();
                if let Some(pos) = self
                    .state
                    .agent_config
                    .disabled_hooks
                    .iter()
                    .position(|d| d == &uid)
                {
                    self.state.agent_config.disabled_hooks.remove(pos);
                } else {
                    self.state.agent_config.disabled_hooks.push(uid);
                }
            }
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_toggle_menu_enable_all(&mut self) {
        if self.ui.config_tab == ConfigTab::Tools {
            self.state.agent_config.disabled_tools.clear();
            // 启用全部时同时清除 deferred 状态（双写）
            self.state.agent_config.deferred_tools.clear();
            let mut deferred = match self.deferred_tools.lock() {
                Ok(guard) => guard,
                Err(e) => e.into_inner(),
            };
            deferred.clear();
            drop(deferred);
            if let Ok(mut loaded) = self.session_loaded_deferred.lock() {
                loaded.clear();
            }
            self.show_toast("已启用全部工具", false);
        } else if self.ui.config_tab == ConfigTab::Skills {
            self.state.agent_config.disabled_skills.clear();
            self.show_toast("已启用全部 Skills", false);
        } else if self.ui.config_tab == ConfigTab::Commands {
            self.state.agent_config.disabled_commands.clear();
            self.show_toast("已启用全部命令", false);
        } else if self.ui.config_tab == ConfigTab::Hooks {
            self.state.agent_config.disabled_hooks.clear();
            self.show_toast("已启用全部 Hooks", false);
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_toggle_menu_disable_all(&mut self) {
        if self.ui.config_tab == ConfigTab::Tools {
            self.state.agent_config.disabled_tools = self
                .tool_registry
                .tool_names()
                .iter()
                .map(|n| n.to_string())
                .collect();
            // 禁用全部时清除 deferred 状态（双写）
            self.state.agent_config.deferred_tools.clear();
            let mut deferred = match self.deferred_tools.lock() {
                Ok(guard) => guard,
                Err(e) => e.into_inner(),
            };
            deferred.clear();
            drop(deferred);
            if let Ok(mut loaded) = self.session_loaded_deferred.lock() {
                loaded.clear();
            }
            self.show_toast("已禁用全部工具", false);
        } else if self.ui.config_tab == ConfigTab::Skills {
            self.state.agent_config.disabled_skills = self
                .state
                .loaded_skills
                .iter()
                .map(|s| s.frontmatter.name.clone())
                .collect();
            self.show_toast("已禁用全部 Skills", false);
        } else if self.ui.config_tab == ConfigTab::Commands {
            self.state.agent_config.disabled_commands = self
                .state
                .loaded_commands
                .iter()
                .map(|c| c.frontmatter.name.clone())
                .collect();
            self.show_toast("已禁用全部命令", false);
        } else if self.ui.config_tab == ConfigTab::Hooks {
            if let Ok(manager) = self.hook_manager.lock() {
                self.state.agent_config.disabled_hooks = manager
                    .list_hooks()
                    .iter()
                    .map(|h| h.unique_id.clone())
                    .collect();
            }
            self.show_toast("已禁用全部 Hooks", false);
        }
    }

    pub(in crate::command::chat::app::chat_app) fn update_compact_exempt_toggle(&mut self) {
        let tool_names = self.tool_registry.tool_names();
        if let Some(name) = tool_names.get(self.ui.compact_exempt_idx) {
            let name_str = name.to_string();
            let exempt = &mut self.state.agent_config.compact.micro_compact_exempt_tools;
            if let Some(pos) = exempt.iter().position(|t| t == &name_str) {
                exempt.remove(pos);
            } else {
                exempt.push(name_str);
            }
        }
    }

    /// 豁免压缩工具子列表：鼠标点击选中指定索引
    pub(in crate::command::chat::app::chat_app) fn update_compact_exempt_select(
        &mut self,
        idx: usize,
    ) {
        let total = self.tool_registry.tool_names().len();
        if total > 0 && idx < total {
            self.ui.compact_exempt_idx = idx;
        }
    }
}
