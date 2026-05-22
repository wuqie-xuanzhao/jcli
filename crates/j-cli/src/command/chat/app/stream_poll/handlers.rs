use crate::command::chat::infra::hook::{HookContext, HookEvent};
use crate::command::chat::remote::protocol::{AskOptionInfo, AskQuestionInfo, WsOutbound};
use crate::command::chat::storage::{ChatMessage, MessageRole};
use crate::util::safe_lock;

use crate::command::chat::app::ChatApp;
use crate::command::chat::app::types::{AskAnswer, AskRequest};
use crate::command::chat::app::ui_state::ChatMode;

impl ChatApp {
    pub(in crate::command::chat::app) fn init_ask_mode(&mut self, ask_req: AskRequest) {
        if self.ws_bridge.is_some() {
            let questions: Vec<AskQuestionInfo> = ask_req
                .questions
                .iter()
                .map(|q| AskQuestionInfo {
                    question: q.question.clone(),
                    header: q.header.clone(),
                    options: q
                        .options
                        .iter()
                        .map(|o| AskOptionInfo {
                            label: o.label.clone(),
                            description: o.description.clone(),
                        })
                        .collect(),
                    multi_select: q.multi_select,
                })
                .collect();
            self.broadcast_ws(WsOutbound::AskRequest { questions });
            self.broadcast_ws(WsOutbound::Status {
                state: "ask".to_string(),
            });
        }

        self.ui.tool_ask_mode = true;
        self.ui.tool_ask_questions = ask_req.questions;
        self.ui.tool_ask_current_idx = 0;
        self.ui.tool_ask_answers = Vec::new();
        self.ui.tool_ask_drafts = vec![String::new(); self.ui.tool_ask_questions.len()];
        self.ask_response_tx = Some(ask_req.response_tx);
        self.init_ask_question_state();
        self.ui.tool_interact_selected = 0;
        self.ui.tool_interact_typing = false;
        self.ui.tool_interact_input.clear();
        self.ui.tool_interact_cursor = 0;
    }

    /// 初始化当前 ask 问题的选项状态
    pub fn init_ask_question_state(&mut self) {
        if let Some(q) = self.ui.tool_ask_questions.get(self.ui.tool_ask_current_idx) {
            self.ui.tool_ask_selections = vec![false; q.options.len() + 1];
            self.ui.tool_ask_cursor = 0;

            if q.options.is_empty() {
                self.ui.tool_interact_typing = true;
                self.ui.tool_interact_input.clear();
                self.ui.tool_interact_cursor = 0;
            } else {
                self.ui.tool_interact_typing = false;
                self.ui.tool_interact_input.clear();
                self.ui.tool_interact_cursor = 0;
            }
        }
    }

    /// 提交当前问题的答案，前进到下一题或完成全部
    pub fn ask_submit_answer(&mut self, answer: AskAnswer) {
        let total = self.ui.tool_ask_questions.len();

        if self.ui.tool_ask_current_idx < self.ui.tool_ask_answers.len() {
            self.ui.tool_ask_answers[self.ui.tool_ask_current_idx] = answer;
        } else {
            self.ui.tool_ask_answers.push(answer);
        }

        if self.ui.tool_ask_current_idx + 1 < total {
            self.ui.tool_ask_current_idx += 1;
            self.init_ask_question_state();
        } else {
            let mut answers_map = serde_json::Map::new();
            for (i, q) in self.ui.tool_ask_questions.iter().enumerate() {
                if let Some(ans) = self.ui.tool_ask_answers.get(i) {
                    let val = match ans {
                        AskAnswer::Selected(indices) => {
                            let labels: Vec<&str> = indices
                                .iter()
                                .filter_map(|&idx| q.options.get(idx).map(|o| o.label.as_str()))
                                .collect();
                            labels.join(", ")
                        }
                        AskAnswer::FreeText(text) => text.clone(),
                    };
                    answers_map.insert(q.header.clone(), serde_json::Value::String(val));
                }
            }

            let response = serde_json::json!({ "answers": answers_map }).to_string();
            if let Some(tx) = self.ask_response_tx.take() {
                let _ = tx.send(response);
            }

            self.ui.tool_ask_mode = false;
            self.ui.tool_ask_questions.clear();
            self.ui.tool_ask_current_idx = 0;
            self.ui.tool_ask_answers.clear();
            self.ui.tool_ask_selections.clear();
            self.ui.tool_ask_cursor = 0;
            self.ui.tool_ask_drafts.clear();
            if !self.tool_executor.has_pending_confirm() {
                self.ui.mode = ChatMode::Chat;
            }
        }
    }

    /// 结束加载状态（流式完成或错误）
    pub(in crate::command::chat::app) fn finish_loading(
        &mut self,
        had_error: bool,
        was_cancelled: bool,
    ) {
        if let Some(ref agent) = self.main_agent {
            agent.cancel();
        }

        self.tool_executor.tool_result_tx = None;
        self.main_agent = None;
        self.tool_executor.tools_executing_count = 0;
        self.state.is_loading = false;
        self.ui.last_rendered_streaming_len = 0;
        self.ui.msg_lines_cache = None;
        self.tool_executor.active_tool_calls.clear();

        if was_cancelled {
            let content = {
                let sc = safe_lock(
                    &self.state.streaming_content,
                    "finish_loading::streaming_content",
                );
                sc.clone()
            };
            if !content.is_empty() {
                let cancelled_content = format!("{}\n\n*[已取消]*", content);
                let mut msg = ChatMessage::text(MessageRole::Assistant, cancelled_content);
                let reasoning = safe_lock(
                    &self.state.streaming_reasoning_content,
                    "finish_loading::cancel_reasoning",
                )
                .clone();
                if !reasoning.is_empty() {
                    msg.reasoning_content = Some(reasoning);
                }
                self.push_both_channels(msg);
            }
            safe_lock(
                &self.state.streaming_content,
                "finish_loading::streaming_content_clear",
            )
            .clear();
            if self.ui.auto_scroll {
                self.ui.scroll_offset = usize::MAX;
            }
            self.show_toast("已取消", false);
        } else if !had_error {
            let mut content = {
                let sc = safe_lock(
                    &self.state.streaming_content,
                    "finish_loading::streaming_content_done",
                );
                sc.clone()
            };
            if !content.is_empty() {
                // ★ PostLlmResponse hook（同步，需要返回值来修改 content / abort / retry）
                let hook_result = {
                    let has_hooks = self
                        .hook_manager
                        .lock()
                        .map(|m| m.has_hooks_for(HookEvent::PostLlmResponse))
                        .unwrap_or(false);
                    if has_hooks {
                        let ctx = HookContext {
                            event: HookEvent::PostLlmResponse,
                            assistant_output: Some(content.clone()),
                            messages: Some(
                                safe_lock(&self.context_messages, "PostLlmResponse::ctx_msgs")
                                    .clone(),
                            ),
                            model: self.active_provider().map(|p| p.model.clone()),
                            session_id: Some(self.session_id.clone()),
                            cwd: std::env::current_dir()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|_| ".".to_string()),
                            ..Default::default()
                        };
                        if let Ok(manager) = self.hook_manager.lock() {
                            manager.execute(
                                HookEvent::PostLlmResponse,
                                ctx,
                                &self.state.agent_config.disabled_hooks,
                            )
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some(result) = hook_result {
                    if let Some(ref sys_msg) = result.system_message {
                        self.show_toast(sys_msg, false);
                    }
                    if result.is_stop() {
                        safe_lock(
                            &self.state.streaming_content,
                            "finish_loading::hook_aborted",
                        )
                        .clear();
                        if let Some(feedback) = result.retry_feedback {
                            self.show_toast(format!("纠查官拦截: {}", feedback), true);
                            self.send_message_internal(feedback);
                        } else {
                            self.show_toast("回复被 hook 拦截", true);
                        }
                        return;
                    }
                    if let Some(new_msg) = result.assistant_output {
                        content = new_msg;
                    }
                }

                let mut msg = ChatMessage::text(MessageRole::Assistant, content);
                // 读取流式 reasoning_content，保存到历史消息以便后续渲染 thinking 区块
                let reasoning = safe_lock(
                    &self.state.streaming_reasoning_content,
                    "finish_loading::reasoning_content",
                )
                .clone();
                if !reasoning.is_empty() {
                    msg.reasoning_content = Some(reasoning);
                }
                self.push_both_channels(msg);
                safe_lock(
                    &self.state.streaming_content,
                    "finish_loading::streaming_content_done_clear",
                )
                .clear();
                safe_lock(
                    &self.state.streaming_reasoning_content,
                    "finish_loading::reasoning_content_clear",
                )
                .clear();
                self.show_toast("回复完成 ✓", false);
            }
            if self.ui.auto_scroll {
                self.ui.scroll_offset = usize::MAX;
            }
        } else {
            safe_lock(
                &self.state.streaming_content,
                "finish_loading::streaming_content_error",
            )
            .clear();
        }

        // 直接从 context_messages 持久化到 transcript.jsonl。
        // push_both 刚推入的消息在 context_messages 中，persist_new_messages 直接读取。
        self.persist_new_messages();
        self.persist_new_display_messages();

        // 检查排队的任务
        let next_task = {
            let mut tasks = safe_lock(&self.state.queued_tasks, "finish_loading::queued_tasks");
            if !tasks.is_empty() {
                Some(tasks.remove(0))
            } else {
                None
            }
        };
        if let Some(task_text) = next_task {
            self.send_message_internal(task_text);
        }
    }
}

pub(super) fn extract_tool_description(tool_name: &str, arguments: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;

    match tool_name {
        // Shell：提取 description 字段
        "Shell" => parsed.get("description")?.as_str().map(|s| s.to_string()),
        // 文件工具：提取 path / file_path 作为描述
        "Read" | "Write" | "Edit" | "Glob" | "Grep" => parsed
            .get("path")
            .or_else(|| parsed.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}
