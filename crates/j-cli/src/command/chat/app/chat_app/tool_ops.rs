use super::ChatApp;

impl ChatApp {
    /// 仅取消工具执行，不取消整个流式请求
    pub fn cancel_tools_only(&mut self) {
        self.tool_executor.cancel();
        self.tool_executor.tools_executing_count = 0;
        self.tool_executor.active_tool_calls.clear();
        self.tool_executor.pending_tool_execution = false;
        self.show_toast("工具已取消", false);
    }

    /// 取消当前流式请求
    ///
    /// 立即执行 finish_loading() 清除加载状态，不等 agent 线程响应取消信号。
    /// 同时停止所有 teammates，确保 Esc 按键后 UI 瞬间恢复可交互状态。
    pub fn cancel_stream(&mut self) {
        // 停止所有 teammates
        if let Ok(mut mgr) = self.teammate_manager.lock() {
            mgr.stop_all();
        }
        self.finish_loading(false, true);
    }

    /// 执行当前待处理工具（兼容旧接口）
    pub fn execute_pending_tool(&mut self) {
        if let Some(new_mode) = self.tool_executor.execute_current(&self.tool_registry) {
            self.ui.mode = new_mode;
        } else {
            self.reset_tool_confirm_interact_state();
        }
    }

    /// 拒绝当前待处理工具（兼容旧接口）
    pub fn reject_pending_tool(&mut self, reason: &str) {
        if let Some(new_mode) = self.tool_executor.reject_current(reason) {
            self.ui.mode = new_mode;
        } else {
            self.reset_tool_confirm_interact_state();
        }
    }

    /// 允许并执行当前待处理工具（兼容旧接口）
    pub fn allow_and_execute_pending_tool(&mut self) {
        if let Some(new_mode) = self
            .tool_executor
            .allow_and_execute(&self.tool_registry, &mut self.jcli_config)
        {
            self.ui.mode = new_mode;
        } else {
            self.reset_tool_confirm_interact_state();
        }
    }

    pub(super) fn reset_tool_confirm_interact_state(&mut self) {
        self.ui.tool_interact_selected = 0;
        self.ui.tool_interact_typing = false;
        self.ui.tool_interact_input.clear();
        self.ui.tool_interact_cursor = 0;
    }
}
