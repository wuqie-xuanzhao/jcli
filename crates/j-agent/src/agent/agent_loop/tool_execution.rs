use std::mem::take;
use std::sync::{Arc, Mutex, mpsc};

use crate::agent::tool_processor::{ToolCallResult, clear_channels, flush_streaming_as_message};
use crate::chat_error::ChatError;
use crate::context::compact::{self, AutoCompactParams, CompactConfig, InvokedSkillsMap};
use crate::infra::hook::{HookContext, HookEvent, HookManager};
use crate::message_types::StreamMsg;
use crate::storage::{ChatMessage, MessageRole, ModelProvider, SessionMetrics};
use crate::util::safe_lock;

use super::compact::push_compact_tool_messages;

// ============================================================
// Streaming flush helper
// ============================================================

/// 从 `assistant_reasoning` 中提取 reasoning 并 flush 流式内容为完整消息。
pub(super) fn flush_streaming_content(
    streaming_content: &Arc<Mutex<String>>,
    streaming_reasoning_content: &Arc<Mutex<String>>,
    messages: &mut Vec<ChatMessage>,
    display_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    context_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    assistant_reasoning: &mut String,
) {
    let reasoning: Option<String> = {
        let r = take(assistant_reasoning);
        if r.is_empty() { None } else { Some(r) }
    };
    flush_streaming_as_message(
        streaming_content,
        streaming_reasoning_content,
        messages,
        display_messages,
        context_messages,
        reasoning,
    );
}

// ============================================================
// Tool call result handler: compact + plan context clear
// ============================================================

/// 处理 `process_tool_calls` 返回后的 compact 触发和 plan 清空上下文逻辑。
#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_post_tool_result(
    result: &ToolCallResult,
    messages: &mut Vec<ChatMessage>,
    display_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    context_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    tx: &mpsc::Sender<StreamMsg>,
    compact_enabled: bool,
    provider: &ModelProvider,
    invoked_skills: &InvokedSkillsMap,
    session_id: &str,
    metrics: &mut SessionMetrics,
) {
    if result.compact_requested && compact_enabled {
        let _ = tx.send(StreamMsg::Compacting);
        if let Ok(compact_result) = compact::auto_compact(
            messages,
            &AutoCompactParams {
                provider,
                invoked_skills,
                session_id,
                protected_context: None,
            },
        )
        .await
        {
            clear_channels(display_messages, context_messages);
            push_compact_tool_messages(
                messages,
                display_messages,
                context_messages,
                &compact_result,
            );
            metrics.auto_compact_count += 1;
            let _ = tx.send(StreamMsg::Compacted {
                messages_before: compact_result.messages_before,
            });
        }
    }

    if let Some(ref plan_content) = result.plan_with_context_clear {
        messages.clear();
        if let Ok(mut shared) = display_messages.lock() {
            shared.clear();
        }
        if let Ok(mut shared) = context_messages.lock() {
            shared.clear();
        }
        let plan_msg = ChatMessage::text(
            MessageRole::User,
            format!("以下计划已获批准，请按计划执行：\n\n{}", plan_content),
        );
        messages.push(plan_msg);
    }
}

// ============================================================
// Compact stage handler (micro_compact + auto_compact with hooks)
// ============================================================

/// 处理每轮的 compact 管线：
/// - PreMicroCompact hook → micro_compact → PostMicroCompact hook
/// - token 超阈值 → PreAutoCompact hook → auto_compact → PostAutoCompact hook
#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_compact_stages(
    messages: &mut Vec<ChatMessage>,
    system_prompt: &Option<String>,
    compact_config: &CompactConfig,
    hook_manager: &HookManager,
    disabled_hooks: &[String],
    display_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    context_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    tx: &mpsc::Sender<StreamMsg>,
    provider: &ModelProvider,
    invoked_skills: &InvokedSkillsMap,
    session_id: &str,
    metrics: &mut SessionMetrics,
) {
    if !compact_config.enabled {
        return;
    }

    let mut compact_aborted = false;

    // ★ PreMicroCompact hook
    if hook_manager.has_hooks_for(HookEvent::PreMicroCompact) {
        let ctx = HookContext {
            event: HookEvent::PreMicroCompact,
            messages: Some(messages.clone()),
            model: Some(provider.model.clone()),
            session_id: Some(session_id.to_string()),
            ..Default::default()
        };
        if let Some(result) = hook_manager.execute(HookEvent::PreMicroCompact, ctx, disabled_hooks)
            && result.is_stop()
        {
            compact_aborted = true;
        }
    }

    if !compact_aborted {
        compact::micro_compact(
            messages,
            compact_config.keep_recent,
            &compact_config.micro_compact_exempt_tools,
        );
        metrics.micro_compact_count += 1;

        // ★ PostMicroCompact hook
        if hook_manager.has_hooks_for(HookEvent::PostMicroCompact) {
            let ctx = HookContext {
                event: HookEvent::PostMicroCompact,
                messages: Some(messages.clone()),
                session_id: Some(session_id.to_string()),
                ..Default::default()
            };
            if let Some(result) =
                hook_manager.execute(HookEvent::PostMicroCompact, ctx, disabled_hooks)
                && let Some(new_msgs) = result.messages
            {
                *messages = new_msgs;
            }
        }

        if compact::estimate_tokens(messages) > compact_config.effective_token_threshold() {
            let mut protected_context: Option<String> = None;

            // ★ PreAutoCompact hook
            if hook_manager.has_hooks_for(HookEvent::PreAutoCompact) {
                let ctx = HookContext {
                    event: HookEvent::PreAutoCompact,
                    messages: Some(messages.clone()),
                    system_prompt: system_prompt.clone(),
                    model: Some(provider.model.clone()),
                    session_id: Some(session_id.to_string()),
                    ..Default::default()
                };
                if let Some(result) =
                    hook_manager.execute(HookEvent::PreAutoCompact, ctx, disabled_hooks)
                {
                    if result.is_stop() {
                        compact_aborted = true;
                    }
                    if let Some(ac) = result.additional_context {
                        protected_context = Some(ac);
                    }
                }
            }

            if !compact_aborted {
                let _ = tx.send(StreamMsg::Compacting);
                match compact::auto_compact(
                    messages,
                    &AutoCompactParams {
                        provider,
                        invoked_skills,
                        session_id,
                        protected_context: protected_context.as_deref(),
                    },
                )
                .await
                {
                    Err(e) => {
                        crate::util::log::write_error_log(
                            "agent_loop",
                            &format!("auto_compact failed: {}", e),
                        );
                    }
                    Ok(compact_result) => {
                        clear_channels(display_messages, context_messages);
                        push_compact_tool_messages(
                            messages,
                            display_messages,
                            context_messages,
                            &compact_result,
                        );
                        metrics.auto_compact_count += 1;
                        let _ = tx.send(StreamMsg::Compacted {
                            messages_before: compact_result.messages_before,
                        });
                        // ★ PostAutoCompact hook
                        if hook_manager.has_hooks_for(HookEvent::PostAutoCompact) {
                            let ctx = HookContext {
                                event: HookEvent::PostAutoCompact,
                                messages: Some(messages.clone()),
                                session_id: Some(session_id.to_string()),
                                ..Default::default()
                            };
                            if let Some(hook_result) = hook_manager.execute(
                                HookEvent::PostAutoCompact,
                                ctx,
                                disabled_hooks,
                            ) && let Some(new_msgs) = hook_result.messages
                            {
                                *messages = new_msgs;
                                crate::agent::tool_processor::sync_context_full(
                                    display_messages,
                                    context_messages,
                                    messages,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// PreLlmRequest hook handler
// ============================================================

/// 处理 PreLlmRequest hook。返回 true 表示应中止（stop）。
pub(super) fn handle_pre_llm_hook(
    hook_manager: &HookManager,
    messages: &mut Vec<ChatMessage>,
    system_prompt: &mut Option<String>,
    provider: &ModelProvider,
    session_id: &str,
    disabled_hooks: &[String],
    tx: &mpsc::Sender<StreamMsg>,
) -> bool {
    if !hook_manager.has_hooks_for(HookEvent::PreLlmRequest) {
        return false;
    }

    use std::env::current_dir;
    let ctx = HookContext {
        event: HookEvent::PreLlmRequest,
        messages: Some(messages.clone()),
        system_prompt: system_prompt.clone(),
        model: Some(provider.model.clone()),
        session_id: Some(session_id.to_string()),
        cwd: current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string()),
        ..Default::default()
    };

    if let Some(result) = hook_manager.execute(HookEvent::PreLlmRequest, ctx, disabled_hooks) {
        if result.is_stop() {
            let _ = tx.send(StreamMsg::Error(crate::chat_error::ChatError::HookAborted));
            return true;
        }
        if let Some(new_msgs) = result.messages {
            *messages = new_msgs;
        }
        if let Some(new_prompt) = result.system_prompt {
            *system_prompt = Some(new_prompt);
        }
        if let Some(inject) = result.inject_messages {
            messages.extend(inject);
        }
    }

    false
}

// ============================================================
// Request log builder
// ============================================================

/// 构建请求日志字符串（供 `write_info_log("Chat 请求", ...)` 使用）。
pub(super) fn build_request_log(
    messages: &[ChatMessage],
    system_prompt: &Option<String>,
) -> String {
    let mut log_content = String::new();
    if let Some(system_prompt) = system_prompt {
        log_content.push_str(&format!("[System] {}\n", system_prompt));
    }
    for msg in messages {
        match msg.role {
            MessageRole::Assistant => {
                if !msg.content.is_empty() {
                    log_content.push_str(&format!("[Assistant] {}\n", msg.content));
                }
                if let Some(ref tool_calls) = msg.tool_calls {
                    for tool_call in tool_calls {
                        log_content.push_str(&format!(
                            "[Assistant/ToolCall] {}: {}\n",
                            tool_call.name, tool_call.arguments
                        ));
                    }
                }
            }
            MessageRole::Tool => {
                let id = msg.tool_call_id.as_deref().unwrap_or("?");
                let tool_name = msg
                    .tool_calls
                    .as_ref()
                    .and_then(|tool_calls| tool_calls.first())
                    .map(|tool_call| tool_call.name.as_str())
                    .unwrap_or("unknown");
                log_content.push_str(&format!(
                    "[Tool/Result({} with id `{}`)] result:\n{}\n",
                    tool_name, id, msg.content
                ));
            }
            MessageRole::User => {
                log_content.push_str(&format!("[User] {}\n", msg.content));
            }
            other => {
                log_content.push_str(&format!("[{}] {}\n", other, msg.content));
            }
        }
    }
    log_content
}

// ============================================================
// Token estimation + stats logging
// ============================================================

/// 更新上下文 token 估算值并记录本轮消息统计。
pub(super) fn update_token_and_log_stats(
    messages: &[ChatMessage],
    estimated_context_tokens: &Arc<Mutex<usize>>,
    round_idx: usize,
    supports_vision: bool,
    context_tokens_peak: &mut usize,
) {
    let tokens = compact::estimate_tokens(messages);
    if let Ok(mut ct) = estimated_context_tokens.lock() {
        *ct = tokens;
    }
    if tokens > *context_tokens_peak {
        *context_tokens_peak = tokens;
    }

    let has_images = messages
        .iter()
        .any(|m| m.images.as_ref().is_some_and(|imgs| !imgs.is_empty()));
    crate::util::log::write_info_log(
        "agent_loop",
        &format!(
            "第 {} 轮请求: messages={}, has_images={}, supports_vision={}",
            round_idx,
            messages.len(),
            has_images,
            supports_vision
        ),
    );
}

// ============================================================
// Sanitize messages (clean orphan tool_call/tool_result pairs)
// ============================================================

/// 清理孤立 tool_call/tool_result 配对。
pub(super) fn sanitize_messages_loop(messages: &mut Vec<ChatMessage>) {
    let before = messages.len();
    let cleaned = crate::agent::api::sanitize_messages(messages);
    let changed = cleaned.len() != before
        || cleaned.iter().zip(messages.iter()).any(|(a, b)| {
            let a_count = a.tool_calls.as_ref().map(|tc| tc.len()).unwrap_or(0);
            let b_count = b.tool_calls.as_ref().map(|tc| tc.len()).unwrap_or(0);
            a_count != b_count
        });
    if changed {
        *messages = cleaned;
        crate::util::log::write_info_log(
            "agent_loop",
            &format!(
                "已就地修复孤立 tool_call/tool_result：{} → {} 条消息",
                before,
                messages.len()
            ),
        );
    }
}

// ============================================================
// Buffer clearing helper
// ============================================================

/// 清空流式内容缓冲（每轮开始时调用）。
pub(super) fn clear_streaming_buffers(
    streaming_content: &Arc<Mutex<String>>,
    streaming_reasoning_content: &Arc<Mutex<String>>,
) {
    safe_lock(streaming_content, "agent::streaming_content_clear").clear();
    safe_lock(
        streaming_reasoning_content,
        "agent::streaming_reasoning_clear",
    )
    .clear();
}

// ============================================================
// Tools info logging

// ============================================================
// End-of-session metrics writer
// ============================================================

/// 在 agent loop 结束时写出 session metrics。
pub(super) fn write_end_metrics(
    mut metrics: SessionMetrics,
    session_id: &str,
    context_tokens_peak: usize,
    sub_agent_metrics: &Arc<Mutex<crate::tools::derived_shared::SubAgentMetrics>>,
    invoked_skills: &InvokedSkillsMap,
    tx: &mpsc::Sender<StreamMsg>,
) {
    use std::time::{SystemTime, UNIX_EPOCH};

    metrics.session_end_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    metrics.estimated_context_tokens_peak = context_tokens_peak;
    metrics.skill_loads = {
        if let Ok(skills) = invoked_skills.lock() {
            skills.keys().cloned().collect()
        } else {
            vec![]
        }
    };

    if let Ok(sub) = sub_agent_metrics.lock() {
        metrics.total_llm_calls += sub.total_llm_calls;
        metrics.total_tool_calls += sub.total_tool_calls;
        metrics.total_input_tokens += sub.total_input_tokens;
        metrics.total_output_tokens += sub.total_output_tokens;
        metrics.total_llm_elapsed_ms += sub.total_llm_elapsed_ms;
        metrics.total_tool_elapsed_ms += sub.total_tool_elapsed_ms;
        metrics
            .ttft_ms_per_call
            .extend(&sub.llm_elapsed_ms_per_call);
    }

    let _ = crate::storage::write_session_metrics(session_id, &metrics);
    let metrics_event = crate::storage::SessionEvent::Metrics { metrics };
    let _ = crate::storage::append_session_event(session_id, &metrics_event);

    let _ = tx.send(StreamMsg::Done);
}

/// `handle_tool_id_mismatch` 的返回动作
pub(super) enum IdMismatchAction {
    /// compact 成功，重试本轮
    ContinueRound,
    /// 出错，从 agent loop 返回
    Return,
    /// 无需处理（needs_compact_for_tool_id_mismatch 为 false）
    NoMismatch,
}

/// 处理 tool_call_id 不一致错误：压缩上下文后重试本轮。
#[allow(clippy::too_many_arguments)]
pub(super) async fn tool_id_mismatch_handler(
    needs_compact: bool,
    messages: &mut Vec<ChatMessage>,
    streaming_content: &Arc<Mutex<String>>,
    streaming_reasoning_content: &Arc<Mutex<String>>,
    compact_config: &CompactConfig,
    display_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    context_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    tx: &mpsc::Sender<StreamMsg>,
    provider: &ModelProvider,
    invoked_skills: &InvokedSkillsMap,
    session_id: &str,
    metrics: &mut SessionMetrics,
) -> IdMismatchAction {
    if !needs_compact {
        return IdMismatchAction::NoMismatch;
    }

    crate::util::log::write_info_log(
        "agent_loop",
        "tool_call_id 不一致错误：将执行 auto_compact 压缩上下文后重试",
    );
    safe_lock(streaming_content, "agent::tool_id_error_clear").clear();
    safe_lock(
        streaming_reasoning_content,
        "agent::tool_id_error_reason_clear",
    )
    .clear();

    if compact_config.enabled {
        let _ = tx.send(StreamMsg::Compacting);
        match compact::auto_compact(
            messages,
            &AutoCompactParams {
                provider,
                invoked_skills,
                session_id,
                protected_context: None,
            },
        )
        .await
        {
            Err(e) => {
                crate::util::log::write_error_log(
                    "agent_loop",
                    &format!("tool_call_id 恢复时 auto_compact 失败: {}", e),
                );
                let _ = tx.send(StreamMsg::Error(ChatError::Other(format!(
                    "消息历史损坏且自动修复失败: {}",
                    e
                ))));
                IdMismatchAction::Return
            }
            Ok(compact_result) => {
                clear_channels(display_messages, context_messages);
                push_compact_tool_messages(
                    messages,
                    display_messages,
                    context_messages,
                    &compact_result,
                );
                metrics.auto_compact_count += 1;
                let _ = tx.send(StreamMsg::Compacted {
                    messages_before: compact_result.messages_before,
                });
                IdMismatchAction::ContinueRound
            }
        }
    } else {
        let _ = tx.send(StreamMsg::Error(ChatError::Other(
            "消息历史中 tool_call_id 不一致，且 compact 未启用，无法自动恢复".to_string(),
        )));
        IdMismatchAction::Return
    }
}
