use super::ChatApp;

impl ChatApp {
    /// 获取当前活跃的 provider
    pub fn active_provider(&self) -> Option<&crate::command::chat::storage::ModelProvider> {
        if self.state.agent_config.providers.is_empty() {
            return None;
        }
        let idx = self
            .state
            .agent_config
            .active_index
            .min(self.state.agent_config.providers.len() - 1);
        Some(&self.state.agent_config.providers[idx])
    }

    /// 获取当前模型名称
    pub fn active_model_name(&self) -> String {
        self.active_provider()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "未配置".to_string())
    }

    pub fn switch_model(&mut self) {
        if let Some(sel) = self.ui.model_list_state.selected() {
            self.state.agent_config.active_index = sel;
            let _ = crate::command::chat::storage::save_agent_config(&self.state.agent_config);
            let name = self.active_model_name();
            self.show_toast(format!("已切换到: {}", name), false);
        }
        self.ui.mode = crate::command::chat::app::ui_state::ChatMode::Chat;
    }
}
