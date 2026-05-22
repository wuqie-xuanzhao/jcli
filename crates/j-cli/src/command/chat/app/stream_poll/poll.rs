use crate::command::chat::infra::hook::{HookContext, HookEvent};
use crate::command::chat::remote::protocol::WsOutbound;
use crate::util::log::write_info_log;
use crate::util::safe_lock;

use super::handlers::extract_tool_description;
use crate::command::chat::app::ChatApp;
use crate::command::chat::app::action::Action;
use crate::command::chat::app::types::{
    PlanDecision, StreamMsg, ToolCallStatus, ToolExecStatus, ToolResultMsg,
};
use crate::command::chat::app::ui_state::ChatMode;

impl ChatApp {
    /// 处理后台流式消息（在主循环中每帧调用）
    /// 返回需要通过 update() 分发的 Actions 列表
    pub fn poll_stream_actions(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();

        // ★ 双通道增量检测（UI 和 LLM context 完全分离）
        //
        // **设计说明**：
        // - `display_messages`：UI 渲染数据源（干净文本 + sender_name 字段），
        //   `build_message_lines_incremental` 直接读取。
        // - `context_messages`：LLM context 数据源（XML 前缀，如 `<Teammate@Frontend>text</Teammate@Frontend>`），
        //   `build_api_messages` 直接读取，LLM 据此区分消息来源。也是持久化（transcript.jsonl）的数据源。
        //
        // 两个通道数据独立：display 用干净文本给 UI 渲染，context 用 XML 包裹给 LLM 识别来源。
        // 两者通过 `push_both` / 各自推送逻辑保持同步写入。
        {
            let display = safe_lock(&self.display_messages, "poll::display_msgs");
            let new_count = display.len();
            if new_count < self.display_read_offset {
                self.display_read_offset = 0;
            }
            if new_count > self.display_read_offset {
                self.display_read_offset = new_count;
                self.ui.msg_lines_cache = None;
                if self.ui.auto_scroll && self.state.is_loading {
                    self.ui.scroll_offset = usize::MAX;
                }
            }
        }

        if self.main_agent.is_none() {
            return actions;
        }

        // 如果在 ToolConfirm 模式，仍然需要轮询工具执行结果
        if self.ui.mode == ChatMode::ToolConfirm {
            let completed = self.tool_executor.poll_results();
            for (id, name, output, is_error) in completed {
                self.broadcast_ws(WsOutbound::ToolResult {
                    id,
                    name,
                    output,
                    is_error,
                });
            }
            if let Some(ref rx) = self.ask_request_rx
                && let Ok(ask_req) = rx.try_recv()
            {
                self.init_ask_mode(ask_req);
                self.ui.msg_lines_cache = None;
            }
            return actions;
        }

        // 如果上一帧设置了 pending_tool_execution，本帧才真正执行
        if self.tool_executor.pending_tool_execution {
            self.tool_executor.pending_tool_execution = false;

            if self.ws_bridge.is_some() {
                for tc in &self.tool_executor.active_tool_calls {
                    self.broadcast_ws(WsOutbound::ToolCall {
                        id: tc.tool_call_id.clone(),
                        name: tc.tool_name.clone(),
                        arguments: tc.arguments.clone(),
                    });
                }
            }

            for tc in &mut self.tool_executor.active_tool_calls {
                if let ToolExecStatus::Failed(ref msg) = tc.status
                    && let Some(ref tx) = self.tool_executor.tool_result_tx
                {
                    let final_msg = {
                        let has_hooks = self
                            .hook_manager
                            .lock()
                            .map(|m| m.has_hooks_for(HookEvent::PostToolExecutionFailure))
                            .unwrap_or(false);
                        if has_hooks {
                            let ctx = HookContext {
                                event: HookEvent::PostToolExecutionFailure,
                                tool_name: Some(tc.tool_name.clone()),
                                tool_error: Some(msg.clone()),
                                session_id: Some(self.session_id.clone()),
                                cwd: std::env::current_dir()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|_| ".".to_string()),
                                ..Default::default()
                            };
                            if let Ok(manager) = self.hook_manager.lock() {
                                if let Some(result) = manager.execute(
                                    HookEvent::PostToolExecutionFailure,
                                    ctx,
                                    &self.state.agent_config.disabled_hooks,
                                ) {
                                    result.tool_error.unwrap_or_else(|| msg.clone())
                                } else {
                                    msg.clone()
                                }
                            } else {
                                msg.clone()
                            }
                        } else {
                            msg.clone()
                        }
                    };
                    let _ = tx.send(ToolResultMsg {
                        tool_call_id: tc.tool_call_id.clone(),
                        result: final_msg,
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    });
                }
            }

            let first_confirm_idx = self
                .tool_executor
                .active_tool_calls
                .iter()
                .position(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm));

            if let Some(idx) = first_confirm_idx {
                self.tool_executor.pending_tool_idx = idx;
                self.tool_executor.tool_confirm_entered_at = std::time::Instant::now();
                self.tool_executor.execute_batch(&self.tool_registry);
                self.ui.tool_interact_selected = 0;
                self.ui.tool_interact_typing = false;
                self.ui.tool_interact_input.clear();
                self.ui.tool_interact_cursor = 0;
                self.ui.tool_ask_mode = false;
                self.ui.tool_ask_questions.clear();
                self.ui.tool_ask_current_idx = 0;
                self.ui.tool_ask_answers.clear();
                self.ui.tool_ask_selections.clear();
                self.ui.tool_ask_cursor = 0;
                actions.push(Action::EnterMode(ChatMode::ToolConfirm));
                write_info_log(
                    "poll_stream",
                    &format!(
                        "进入 ToolConfirm 模式, pending_tool_idx={}, active_tool_calls={}, tools_executing_count={}",
                        self.tool_executor.pending_tool_idx,
                        self.tool_executor.active_tool_calls.len(),
                        self.tool_executor.tools_executing_count,
                    ),
                );
            } else {
                write_info_log(
                    "poll_stream",
                    &format!(
                        "无需确认的工具, 直接执行, active_tool_calls={}",
                        self.tool_executor.active_tool_calls.len(),
                    ),
                );
                self.tool_executor.execute_batch(&self.tool_registry);
            }
            return actions;
        }

        // 轮询后台工具执行结果
        let completed = self.tool_executor.poll_results();
        for (id, name, output, is_error) in completed {
            self.broadcast_ws(WsOutbound::ToolResult {
                id,
                name,
                output,
                is_error,
            });
        }

        // 轮询 ask 工具请求
        if let Some(ref rx) = self.ask_request_rx
            && let Ok(ask_req) = rx.try_recv()
        {
            self.init_ask_mode(ask_req);
            actions.push(Action::EnterMode(ChatMode::ToolConfirm));
            self.ui.msg_lines_cache = None;
            return actions;
        }

        if let Some(ref agent) = self.main_agent {
            let msgs = agent.poll();
            for msg in msgs {
                match msg {
                    StreamMsg::Chunk => {
                        actions.push(Action::StreamChunk);
                    }
                    StreamMsg::ToolCallRequest(tool_calls) => {
                        self.tool_executor.active_tool_calls.clear();
                        self.tool_executor.pending_tool_idx = 0;

                        for mut tc in tool_calls {
                            // ★ PreToolExecution hook（同步，需要返回值）
                            {
                                let has_hooks = self
                                    .hook_manager
                                    .lock()
                                    .map(|m| m.has_hooks_for(HookEvent::PreToolExecution))
                                    .unwrap_or(false);
                                if has_hooks {
                                    let ctx = HookContext {
                                        event: HookEvent::PreToolExecution,
                                        tool_name: Some(tc.name.clone()),
                                        tool_arguments: Some(tc.arguments.clone()),
                                        session_id: Some(self.session_id.clone()),
                                        cwd: std::env::current_dir()
                                            .map(|p| p.display().to_string())
                                            .unwrap_or_else(|_| ".".to_string()),
                                        ..Default::default()
                                    };
                                    if let Ok(manager) = self.hook_manager.lock()
                                        && let Some(result) = manager.execute(
                                            HookEvent::PreToolExecution,
                                            ctx,
                                            &self.state.agent_config.disabled_hooks,
                                        )
                                    {
                                        if result.is_skip() {
                                            self.tool_executor.active_tool_calls.push(
                                                ToolCallStatus {
                                                    tool_call_id: tc.id.clone(),
                                                    tool_name: tc.name.clone(),
                                                    arguments: tc.arguments.clone(),
                                                    confirm_message: format!(
                                                        "🚫 {} 被 hook 跳过",
                                                        tc.name
                                                    ),
                                                    status: ToolExecStatus::Failed(
                                                        "该工具调用被 hook 跳过".to_string(),
                                                    ),
                                                    tool_description: extract_tool_description(
                                                        &tc.name,
                                                        &tc.arguments,
                                                    ),
                                                },
                                            );
                                            continue;
                                        }
                                        if let Some(new_args) = result.tool_arguments {
                                            tc.arguments = new_args;
                                        }
                                    }
                                }
                            }

                            if self.jcli_config.is_denied(&tc.name, &tc.arguments) {
                                self.tool_executor.active_tool_calls.push(ToolCallStatus {
                                    tool_call_id: tc.id.clone(),
                                    tool_name: tc.name.clone(),
                                    arguments: tc.arguments.clone(),
                                    confirm_message: format!(
                                        "🚫 {} 被 .jcli/ 权限配置拒绝",
                                        tc.name
                                    ),
                                    status: ToolExecStatus::Failed(
                                        "该命令被 .jcli/ 权限配置拒绝".to_string(),
                                    ),
                                    tool_description: extract_tool_description(
                                        &tc.name,
                                        &tc.arguments,
                                    ),
                                });
                                continue;
                            }

                            let sandbox_outside = self.sandbox.is_outside(&tc.name, &tc.arguments);
                            let confirm_msg = if sandbox_outside {
                                self.sandbox.outside_message(&tc.name, &tc.arguments)
                            } else if let Some(tool) = self.tool_registry.get(&tc.name) {
                                tool.confirmation_message(&tc.arguments)
                            } else {
                                format!("调用工具 {} 参数: {}", tc.name, tc.arguments)
                            };
                            let tool_needs_confirm = self
                                .tool_registry
                                .get(&tc.name)
                                .map(|t| t.requires_confirmation())
                                .unwrap_or(false);
                            let needs_confirm = (tool_needs_confirm || sandbox_outside)
                                && !self.jcli_config.is_allowed(&tc.name, &tc.arguments)
                                && !self.ui.auto_approve;
                            self.tool_executor.active_tool_calls.push(ToolCallStatus {
                                tool_call_id: tc.id.clone(),
                                tool_name: tc.name.clone(),
                                arguments: tc.arguments.clone(),
                                confirm_message: confirm_msg,
                                status: if needs_confirm {
                                    ToolExecStatus::PendingConfirm
                                } else {
                                    ToolExecStatus::Executing
                                },
                                tool_description: extract_tool_description(&tc.name, &tc.arguments),
                            });
                        }

                        self.tool_executor.pending_tool_execution = true;
                        break;
                    }
                    StreamMsg::Done => {
                        actions.push(Action::StreamDone);
                        break;
                    }
                    StreamMsg::Error(e) => {
                        actions.push(Action::StreamError(e));
                        break;
                    }
                    StreamMsg::Cancelled => {
                        actions.push(Action::StreamCancelled);
                        break;
                    }
                    StreamMsg::Retrying {
                        attempt,
                        max_attempts,
                        delay_ms,
                        error,
                    } => {
                        actions.push(Action::StreamRetrying {
                            attempt,
                            max_attempts,
                            delay_ms,
                            error,
                        });
                    }
                    StreamMsg::Compacting => {
                        actions.push(Action::StreamCompacting);
                    }
                    StreamMsg::Compacted { messages_before } => {
                        actions.push(Action::StreamCompacted { messages_before });
                    }
                }
            }
        }

        actions
    }
}
