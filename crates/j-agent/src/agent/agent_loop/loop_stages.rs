use std::collections::BTreeMap;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use futures::StreamExt;
use rand::Rng;

use crate::agent::api::{build_request_with_tools, call_llm_non_stream, create_llm_client};
use crate::agent::config::{AgentLoopConfig, AgentLoopSharedState};
use crate::agent::retry::{backoff_delay_ms, retry_policy_for};
use crate::agent::tool_processor::{
    ToolCallContext, drain_pending_user_messages, process_tool_calls, push_both,
};
use crate::chat_error::ChatError;
use crate::infra::hook::{HookContext, HookEvent};
use crate::message_types::{StreamMsg, ToolResultMsg};
use crate::storage::{ChatMessage, MessageRole, SessionMetrics, ToolCallItem};
use crate::util::log::{write_error_log, write_info_log};
use crate::util::safe_lock;

use super::compact::StreamingToolCallPart;
use super::tool_execution::{
    IdMismatchAction, build_request_log, clear_streaming_buffers, flush_streaming_content,
    handle_compact_stages, handle_post_tool_result, handle_pre_llm_hook, sanitize_messages_loop,
    tool_id_mismatch_handler, update_token_and_log_stats, write_end_metrics,
};

/// 调试日志中记录的最大 chunk 数量
const DEBUG_LOG_CHUNK_LIMIT: u32 = 3;
/// reasoning 内容日志输出的最小长度阈值
const REASONING_LOG_THRESHOLD: usize = 50;

/// `run_main_agent_loop` 的参数集合（将 6 个独立参数封装为单一结构体）
pub struct MainAgentLoopParams {
    pub config: AgentLoopConfig,
    pub shared: AgentLoopSharedState,
    pub messages: Vec<ChatMessage>,
    pub system_prompt_fn: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    pub tx: mpsc::Sender<StreamMsg>,
    pub tool_result_rx: mpsc::Receiver<ToolResultMsg>,
}

/// 后台 Agent 循环：支持多轮工具调用
pub async fn run_main_agent_loop(params: MainAgentLoopParams) {
    let MainAgentLoopParams {
        config,
        shared,
        mut messages,
        system_prompt_fn,
        tx,
        tool_result_rx,
    } = params;

    let AgentLoopConfig {
        provider,
        max_llm_rounds,
        compact_config,
        hook_manager,
        disabled_hooks,
        cancel_token,
    } = config;

    let AgentLoopSharedState {
        streaming_content,
        streaming_reasoning_content,
        pending_user_messages,
        background_manager: _,
        todo_manager,
        display_messages,
        context_messages,
        estimated_context_tokens,
        invoked_skills,
        session_id,
        derived_system_prompt,
        tool_registry,
        disabled_tools,
        tools_enabled,
        sub_agent_metrics,
        deferred_tools,
        session_loaded_deferred: _,
    } = shared;

    let client = create_llm_client(&provider);

    // ── 指标采集局部变量 ──
    let mut metrics = SessionMetrics {
        session_start_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        ..Default::default()
    };
    let mut context_tokens_peak: usize = 0;

    let tool_ctx = ToolCallContext {
        stream_msg_sender: &tx,
        tool_result_receiver: &tool_result_rx,
        pending_user_messages: &pending_user_messages,
        hook_manager: &hook_manager,
        disabled_hooks: &disabled_hooks,
        supports_vision: provider.supports_vision,
        display_messages: &display_messages,
        context_messages: &context_messages,
        streaming_content: &streaming_content,
        session_id: &session_id,
    };

    let mut final_round_idx: usize = 0;

    'round: for round_idx in 0..max_llm_rounds {
        final_round_idx = round_idx;

        // 每轮开始时动态获取可用工具
        let tools = if tools_enabled {
            let deferred: Vec<String> = match deferred_tools.lock() {
                Ok(guard) => guard.clone(),
                Err(e) => e.into_inner().clone(),
            };
            tool_registry.to_llm_tools_non_deferred(&disabled_tools, &deferred)
        } else {
            vec![]
        };

        write_info_log(
            "agent_loop",
            &format!(
                "========== 第 {} 轮开始 (max={}) ==========",
                round_idx, max_llm_rounds
            ),
        );
        write_info_log(
            "agent_loop",
            &format!(
                "第 {} 轮可用工具: [{}] (count={})",
                round_idx,
                tools
                    .iter()
                    .map(|t| t.function.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                tools.len()
            ),
        );

        // 每轮重新构建 system prompt
        let mut system_prompt = system_prompt_fn();
        {
            if let Ok(mut sp) = derived_system_prompt.lock() {
                *sp = system_prompt.clone();
            }
        }

        // 每轮开始时 drain 用户增量消息
        let pending_count_before = safe_lock(&pending_user_messages, "agent::pending_count").len();
        drain_pending_user_messages(&mut messages, &pending_user_messages);
        if pending_count_before > 0 {
            write_info_log(
                "agent_loop",
                &format!("drain 了 {} 条用户增量消息", pending_count_before),
            );
        }

        // ── 每轮 compact 管线 ──
        handle_compact_stages(
            &mut messages,
            &system_prompt,
            &compact_config,
            &hook_manager,
            &disabled_hooks,
            &display_messages,
            &context_messages,
            &tx,
            &provider,
            &invoked_skills,
            &session_id,
            &mut metrics,
        )
        .await;

        // 检查待办事项（每轮递增计数器）
        todo_manager.increment_turn();

        // 清空流式缓冲
        clear_streaming_buffers(&streaming_content, &streaming_reasoning_content);

        // 记录请求日志
        let log_content = build_request_log(&messages, &system_prompt);
        write_info_log("Chat 请求", &log_content);

        // ★ PreLlmRequest hook
        if handle_pre_llm_hook(
            &hook_manager,
            &mut messages,
            &mut system_prompt,
            &provider,
            &session_id,
            &disabled_hooks,
            &tx,
        ) {
            return;
        }

        // 更新 token 估算 + 消息统计日志
        update_token_and_log_stats(
            &messages,
            &estimated_context_tokens,
            round_idx,
            provider.supports_vision,
            &mut context_tokens_peak,
        );

        // ── 清理孤立 tool_call/tool_result 配对 ──
        // ── 清理孤立 tool_call/tool_result 配对 ──
        sanitize_messages_loop(&mut messages);

        let request = match build_request_with_tools(
            &provider,
            &messages,
            tools.clone(),
            system_prompt.as_deref(),
        ) {
            Ok(req) => {
                for (i, m) in messages.iter().enumerate() {
                    if let Some(rc) = &m.reasoning_content {
                        write_info_log(
                            "agent_loop",
                            &format!(
                                "messages[{}] role={:?} has reasoning_content len={}",
                                i,
                                m.role,
                                rc.len()
                            ),
                        );
                    }
                }
                write_info_log("agent_loop", "build_request_with_tools 成功");
                req
            }
            Err(e) => {
                let _ = tx.send(StreamMsg::Error(e));
                return;
            }
        };

        // ── 指数退避重试循环 ──
        let mut retry_attempt: u32 = 0;

        'api_retry: loop {
            retry_attempt += 1;
            let call_start = Instant::now();

            // ── 创建流式请求（可重试）──
            write_info_log(
                "agent_loop",
                &format!("开始创建流式请求 (attempt={})...", retry_attempt),
            );
            let mut stream = match client.chat_completion_stream(&request).await {
                Ok(s) => {
                    write_info_log("agent_loop", "流式请求创建成功");
                    s
                }
                Err(e) => {
                    let err = ChatError::from(e);
                    write_error_log("Chat API 流式请求创建", &err.to_string());
                    if let Some(policy) = retry_policy_for(&err)
                        && retry_attempt <= policy.max_attempts
                    {
                        let delay_ms =
                            backoff_delay_ms(retry_attempt, policy.base_ms, policy.cap_ms);
                        write_info_log(
                            "agent_loop",
                            &format!(
                                "流式创建失败，{}ms 后重试 ({}/{})",
                                delay_ms, retry_attempt, policy.max_attempts
                            ),
                        );
                        let _ = tx.send(StreamMsg::Retrying {
                            attempt: retry_attempt,
                            max_attempts: policy.max_attempts,
                            delay_ms,
                            error: err.display_message(),
                        });
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {
                                continue 'api_retry;
                            }
                            _ = cancel_token.cancelled() => {
                                let _ = tx.send(StreamMsg::Cancelled);
                                return;
                            }
                        }
                    }
                    let _ = tx.send(StreamMsg::Error(err));
                    return;
                }
            };

            // ── 读取流式响应 ──
            let mut finish_reason: Option<String> = None;
            let mut assistant_text = String::new();
            let mut assistant_reasoning = String::new();
            let mut active_tool_call_parts: BTreeMap<u32, StreamingToolCallPart> = BTreeMap::new();
            let mut deserialize_failed = false;
            let mut needs_compact_for_tool_id_mismatch = false;
            let mut stream_retriable_error: Option<ChatError> = None;

            let mut received_chunks: u32 = 0;

            'stream: loop {
                tokio::select! {
                    result = stream.next() => {
                        match result {
                            Some(Ok(response)) => {
                                received_chunks += 1;
                                if received_chunks == 1 {
                                    metrics.ttft_ms_per_call.push(call_start.elapsed().as_millis() as u64);
                                }
                                if let Some(ref usage) = response.usage {
                                    metrics.total_input_tokens += usage.prompt_tokens;
                                    metrics.total_output_tokens += usage.completion_tokens;
                                }
                                if received_chunks <= DEBUG_LOG_CHUNK_LIMIT {
                                    let choices_debug: Vec<String> = response.choices.iter().map(|choice| {
                                        format!(
                                            "idx={}, finish_reason={:?}, has_content={}, has_tool_calls={}",
                                            choice.index,
                                            choice.finish_reason,
                                            choice.delta.content.is_some(),
                                            choice.delta.tool_calls.is_some(),
                                        )
                                    }).collect();
                                    write_info_log(
                                        "stream_chunk",
                                        &format!("chunk #{}: choices=[{}]", received_chunks, choices_debug.join("; ")),
                                    );
                                }
                                for choice in &response.choices {
                                    if let Some(ref content) = choice.delta.content {
                                        assistant_text.push_str(content);
                                        let mut stream_buf = safe_lock(&streaming_content, "agent::stream_chunk");
                                        stream_buf.push_str(content);
                                        drop(stream_buf);
                                        let _ = tx.send(StreamMsg::Chunk);
                                    }
                                    if let Some(ref reasoning) = choice.delta.reasoning_content {
                                        assistant_reasoning.push_str(reasoning);
                                        {
                                            let mut reason_buf = safe_lock(&streaming_reasoning_content, "agent::stream_reasoning");
                                            reason_buf.push_str(reasoning);
                                        }
                                        let _ = tx.send(StreamMsg::Chunk);
                                    }
                                    if !assistant_reasoning.is_empty() && choice.delta.reasoning_content.is_some() && assistant_reasoning.len() < REASONING_LOG_THRESHOLD {
                                        write_info_log("agent_loop", &format!("reasoning积累中 len={}", assistant_reasoning.len()));
                                    }
                                    if let Some(ref toolcall_chunks) = choice.delta.tool_calls {
                                        for chunk in toolcall_chunks {
                                            let entry =
                                                active_tool_call_parts.entry(chunk.index).or_insert_with(|| {
                                                    StreamingToolCallPart {
                                                        call_id: chunk.id.clone().unwrap_or_default(),
                                                        function_name: String::new(),
                                                        function_arguments: String::new(),
                                                    }
                                                });
                                            if entry.call_id.is_empty()
                                                && let Some(ref id) = chunk.id {
                                                    entry.call_id = id.clone();
                                                }
                                            if let Some(ref tool_function) = chunk.function {
                                                if let Some(ref name) = tool_function.name {
                                                    entry.function_name.push_str(name);
                                                }
                                                if let Some(ref args) = tool_function.arguments {
                                                    entry.function_arguments.push_str(args);
                                                }
                                            }
                                        }
                                    }
                                    if let Some(ref finish_reason_val) = choice.finish_reason {
                                        finish_reason = Some(finish_reason_val.clone());
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                let error_str = e.to_string();
                                write_error_log("Chat API 流式响应 error", &error_str);
                                let err = ChatError::from(e);
                                if matches!(err, ChatError::StreamDeserialize(_))
                                    || error_str.contains("missing field `index`")
                                    || error_str.contains("tool_calls")
                                {
                                    write_info_log(
                                        "Chat API 流式响应",
                                        &format!("检测到反序列化错误，将 fallback 到非流式: {}", err),
                                    );
                                    deserialize_failed = true;
                                    break 'stream;
                                }
                                if matches!(&err, ChatError::ApiBadRequest(msg) if msg.contains("tool_call_id")) {
                                    write_error_log(
                                        "Chat API 流式响应",
                                        &format!("检测到 tool_call_id 不一致错误，将压缩上下文后重试: {}", err),
                                    );
                                    needs_compact_for_tool_id_mismatch = true;
                                    break 'stream;
                                }
                                if retry_policy_for(&err).is_some() {
                                    stream_retriable_error = Some(err);
                                    break 'stream;
                                }
                                write_error_log("Chat API 流式响应（不可重试）", &err.to_string());
                                let _ = tx.send(StreamMsg::Error(err));
                                return;
                            }
                            None => {
                                write_info_log("agent_loop", "流式结束 (stream returned None)");
                                break 'stream;
                            }
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        let _ = tx.send(StreamMsg::Cancelled);
                        return;
                    }
                }
            }

            // ── 处理 tool_call_id 不一致错误 ──
            match tool_id_mismatch_handler(
                needs_compact_for_tool_id_mismatch,
                &mut messages,
                &streaming_content,
                &streaming_reasoning_content,
                &compact_config,
                &display_messages,
                &context_messages,
                &tx,
                &provider,
                &invoked_skills,
                &session_id,
                &mut metrics,
            )
            .await
            {
                IdMismatchAction::ContinueRound => continue 'round,
                IdMismatchAction::Return => return,
                IdMismatchAction::NoMismatch => {}
            }

            // ── 处理流式读取中途的可重试错误 ──
            if let Some(err) = stream_retriable_error {
                write_error_log("Chat API 流式响应（将重试）", &err.to_string());
                if let Some(policy) = retry_policy_for(&err)
                    && retry_attempt <= policy.max_attempts
                {
                    safe_lock(&streaming_content, "agent::stream_retry_clear").clear();
                    safe_lock(
                        &streaming_reasoning_content,
                        "agent::stream_retry_reason_clear",
                    )
                    .clear();
                    let delay_ms = backoff_delay_ms(retry_attempt, policy.base_ms, policy.cap_ms);
                    write_info_log(
                        "agent_loop",
                        &format!(
                            "流式中断，{}ms 后重试 ({}/{})",
                            delay_ms, retry_attempt, policy.max_attempts
                        ),
                    );
                    let _ = tx.send(StreamMsg::Retrying {
                        attempt: retry_attempt,
                        max_attempts: policy.max_attempts,
                        delay_ms,
                        error: err.display_message(),
                    });
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {
                            continue 'api_retry;
                        }
                        _ = cancel_token.cancelled() => {
                            let _ = tx.send(StreamMsg::Cancelled);
                            return;
                        }
                    }
                }
                let _ = tx.send(StreamMsg::Error(err));
                return;
            }

            // 记录流式回复日志
            if !assistant_text.is_empty() {
                write_info_log("Sprite 回复", &assistant_text);
            }

            write_info_log(
                "agent_loop",
                &format!(
                    "流式循环结束: finish_reason={:?}, assistant_text_len={}, active_tool_call_parts={}, deserialize_failed={}",
                    finish_reason,
                    assistant_text.len(),
                    active_tool_call_parts.len(),
                    deserialize_failed
                ),
            );

            // 如果反序列化错误或流式空响应 → fallback 到非流式
            let stream_empty = finish_reason.is_none()
                && assistant_text.is_empty()
                && active_tool_call_parts.is_empty();
            write_info_log(
                "agent_loop",
                &format!(
                    "流式结果分析: stream_empty={}, deserialize_failed={}, received_chunks={}",
                    stream_empty, deserialize_failed, received_chunks
                ),
            );
            if deserialize_failed || stream_empty {
                if stream_empty {
                    write_info_log(
                        "agent_loop",
                        &format!(
                            "流式返回空响应 (chunks={}, finish_reason=None, 无内容)，fallback 到非流式重试",
                            received_chunks
                        ),
                    );
                }
                safe_lock(&streaming_content, "agent::fallback_clear").clear();
                safe_lock(&streaming_reasoning_content, "agent::fallback_reason_clear").clear();

                let fallback_result = loop {
                    let create_fut = call_llm_non_stream(&provider, &request);
                    let result = tokio::select! {
                        result = create_fut => result,
                        _ = cancel_token.cancelled() => {
                            let _ = tx.send(StreamMsg::Cancelled);
                            return;
                        }
                    };
                    match result {
                        Ok(r) => {
                            metrics
                                .ttft_ms_per_call
                                .push(call_start.elapsed().as_millis() as u64);
                            if let Some(ref usage) = r.usage {
                                metrics.total_input_tokens += usage.prompt_tokens;
                                metrics.total_output_tokens += usage.completion_tokens;
                            }
                            metrics.total_llm_calls += 1;
                            metrics.total_llm_elapsed_ms += call_start.elapsed().as_millis() as u64;
                            break r;
                        }
                        Err(e) => {
                            write_error_log("Sprite API fallback 非流式", &e.to_string());
                            if let Some(policy) = retry_policy_for(&e)
                                && retry_attempt <= policy.max_attempts
                            {
                                let delay_ms =
                                    backoff_delay_ms(retry_attempt, policy.base_ms, policy.cap_ms);
                                write_info_log(
                                    "agent_loop",
                                    &format!(
                                        "fallback 非流式失败，{}ms 后重试 ({}/{})",
                                        delay_ms, retry_attempt, policy.max_attempts
                                    ),
                                );
                                let _ = tx.send(StreamMsg::Retrying {
                                    attempt: retry_attempt,
                                    max_attempts: policy.max_attempts,
                                    delay_ms,
                                    error: e.display_message(),
                                });
                                tokio::select! {
                                    _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {
                                        retry_attempt += 1;
                                        continue;
                                    }
                                    _ = cancel_token.cancelled() => {
                                        let _ = tx.send(StreamMsg::Cancelled);
                                        return;
                                    }
                                }
                            }
                            let _ = tx.send(StreamMsg::Error(e));
                            return;
                        }
                    }
                };

                write_info_log(
                    "agent_loop",
                    &format!(
                        "fallback 非流式结果: has_tool_calls={}, has_content={}, finish_reason={:?}",
                        fallback_result.has_tool_calls(),
                        fallback_result
                            .content
                            .as_ref()
                            .map(|content| content.len())
                            .unwrap_or(0),
                        fallback_result.finish_reason
                    ),
                );

                if fallback_result.has_tool_calls()
                    && let Some(tool_items) = fallback_result.tool_calls
                {
                    if tool_items.is_empty() {
                        write_info_log("agent_loop", "fallback tool_calls 为空列表，break 'round");
                        break 'round;
                    }
                    let assistant_text: String =
                        fallback_result.content.clone().unwrap_or_default();
                    metrics.total_tool_calls += tool_items.len() as u32;
                    let tool_start = Instant::now();
                    match process_tool_calls(
                        tool_items,
                        assistant_text,
                        &mut messages,
                        &tool_ctx,
                        fallback_result.reasoning_content.clone(),
                    ) {
                        Ok(result) => {
                            metrics.total_tool_elapsed_ms +=
                                tool_start.elapsed().as_millis() as u64;
                            handle_post_tool_result(
                                &result,
                                &mut messages,
                                &display_messages,
                                &context_messages,
                                &tx,
                                compact_config.enabled,
                                &provider,
                                &invoked_skills,
                                &session_id,
                                &mut metrics,
                            )
                            .await;
                            continue 'round;
                        }
                        Err(e) => {
                            write_error_log(
                                "agent_loop",
                                &format!("process_tool_calls failed: {}", e),
                            );
                            return;
                        }
                    }
                }

                if let Some(ref content) = fallback_result.content
                    && !content.is_empty()
                {
                    write_info_log("Sprite 回复", content);
                    let mut stream_buf = safe_lock(&streaming_content, "agent::fallback_content");
                    stream_buf.push_str(content);
                    drop(stream_buf);
                    let _ = tx.send(StreamMsg::Chunk);
                }
                if let Some(ref reason) = fallback_result.finish_reason
                    && !matches!(
                        reason.as_str(),
                        "stop" | "length" | "tool_calls" | "content_filter" | "function_call"
                    )
                    && fallback_result
                        .content
                        .as_deref()
                        .unwrap_or_default()
                        .is_empty()
                {
                    let error_msg = ChatError::AbnormalFinish(reason.clone());
                    write_error_log("Sprite API fallback 非流式", &error_msg.to_string());
                    let _ = tx.send(StreamMsg::Error(error_msg));
                    return;
                }

                let has_pending =
                    !safe_lock(&pending_user_messages, "agent::pending_check_fallback").is_empty();
                write_info_log(
                    "agent_loop",
                    &format!("fallback 正常结束，pending_user_messages={}", has_pending),
                );
                if has_pending {
                    flush_streaming_content(
                        &streaming_content,
                        &streaming_reasoning_content,
                        &mut messages,
                        &display_messages,
                        &context_messages,
                        &mut assistant_reasoning,
                    );
                    write_info_log("agent_loop", "有用户增量消息，continue 'round");
                    continue 'round;
                }
                write_info_log("agent_loop", "无用户增量消息，break 'round (fallback 路径)");

                flush_streaming_content(
                    &streaming_content,
                    &streaming_reasoning_content,
                    &mut messages,
                    &display_messages,
                    &context_messages,
                    &mut assistant_reasoning,
                );
                break 'round;
            }

            // ── 检查流式模式下是否有 tool_calls ──
            let has_tool_calls = !active_tool_call_parts.is_empty();
            write_info_log(
                "agent_loop",
                &format!(
                    "流式路径决策: has_tool_calls={}, finish_reason={:?}",
                    has_tool_calls, finish_reason
                ),
            );

            if has_tool_calls {
                let finish_reason_is_tool_calls = finish_reason.as_deref() == Some("tool_calls");
                if !finish_reason_is_tool_calls {
                    write_info_log(
                        "agent_loop",
                        &format!(
                            "finish_reason={:?} 不是 ToolCalls 但 active_tool_call_parts 非空({})，仍处理工具调用",
                            finish_reason,
                            active_tool_call_parts.len()
                        ),
                    );
                }

                let tool_items: Vec<ToolCallItem> = active_tool_call_parts
                    .into_values()
                    .map(|part| {
                        let id = if part.call_id.is_empty() {
                            let rand_id =
                                format!("call_{:016x}", rand::thread_rng().r#gen::<u64>());
                            write_info_log(
                                "agent_loop",
                                &format!(
                                    "tool_call id 为空（API 未在流式 chunk 中返回），已生成随机 id: {}",
                                    rand_id
                                ),
                            );
                            rand_id
                        } else {
                            part.call_id
                        };
                        ToolCallItem {
                            id,
                            name: part.function_name,
                            arguments: part.function_arguments,
                        }
                    })
                    .collect();

                if tool_items.is_empty() {
                    write_info_log("agent_loop", "流式 tool_items 转换后为空，break 'round");
                    metrics.total_llm_calls += 1;
                    metrics.total_llm_elapsed_ms += call_start.elapsed().as_millis() as u64;
                    break 'round;
                }

                write_info_log(
                    "agent_loop",
                    &format!(
                        "开始处理 {} 个工具调用: [{}]",
                        tool_items.len(),
                        tool_items
                            .iter()
                            .map(|t| t.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                );
                let reasoning_opt = {
                    let r: String = std::mem::take(&mut assistant_reasoning);
                    if r.is_empty() { None } else { Some(r) }
                };
                metrics.total_llm_calls += 1;
                metrics.total_llm_elapsed_ms += call_start.elapsed().as_millis() as u64;
                metrics.total_tool_calls += tool_items.len() as u32;
                let tool_start = Instant::now();
                match process_tool_calls(
                    tool_items,
                    assistant_text,
                    &mut messages,
                    &tool_ctx,
                    reasoning_opt,
                ) {
                    Ok(result) => {
                        metrics.total_tool_elapsed_ms += tool_start.elapsed().as_millis() as u64;
                        handle_post_tool_result(
                            &result,
                            &mut messages,
                            &display_messages,
                            &context_messages,
                            &tx,
                            compact_config.enabled,
                            &provider,
                            &invoked_skills,
                            &session_id,
                            &mut metrics,
                        )
                        .await;
                        continue 'round;
                    }
                    Err(e) => {
                        write_error_log("agent_loop", &format!("process_tool_calls failed: {}", e));
                        return;
                    }
                }
            } else {
                // 正常结束，但如果有用户增量消息则继续循环
                let has_pending =
                    !safe_lock(&pending_user_messages, "agent::pending_check_stream").is_empty();
                write_info_log(
                    "agent_loop",
                    &format!(
                        "LLM 未调用工具 (finish_reason={:?}, text_len={})，pending_user_messages={}",
                        finish_reason,
                        assistant_text.len(),
                        has_pending
                    ),
                );
                if has_pending {
                    flush_streaming_content(
                        &streaming_content,
                        &streaming_reasoning_content,
                        &mut messages,
                        &display_messages,
                        &context_messages,
                        &mut assistant_reasoning,
                    );
                    write_info_log("agent_loop", "有用户增量消息，continue 'round");
                    continue 'round;
                }

                // ★ Stop hook
                if hook_manager.has_hooks_for(HookEvent::Stop) {
                    flush_streaming_content(
                        &streaming_content,
                        &streaming_reasoning_content,
                        &mut messages,
                        &display_messages,
                        &context_messages,
                        &mut assistant_reasoning,
                    );
                    let stop_ctx = HookContext {
                        event: HookEvent::Stop,
                        messages: Some(messages.clone()),
                        system_prompt: system_prompt.clone(),
                        model: Some(provider.model.clone()),
                        user_input: Some(assistant_text.clone()),
                        session_id: Some(session_id.clone()),
                        ..Default::default()
                    };
                    if let Some(result) =
                        hook_manager.execute(HookEvent::Stop, stop_ctx, &disabled_hooks)
                    {
                        if let Some(ref ctx_text) = result.additional_context {
                            let current = system_prompt.unwrap_or_default();
                            system_prompt = Some(format!("{}\n\n{}", current, ctx_text));
                        }
                        if let Some(ref feedback) = result.retry_feedback {
                            write_info_log("Stop hook", &format!("纠查官反馈: {}", feedback));
                            let feedback_msg =
                                ChatMessage::text(MessageRole::User, feedback.clone());
                            messages.push(feedback_msg.clone());
                            push_both(&display_messages, &context_messages, feedback_msg);
                            continue 'round;
                        }
                        if result.is_stop() {
                            let _ = tx.send(StreamMsg::Error(ChatError::HookAborted));
                            return;
                        }
                    }
                }

                write_info_log(
                    "agent_loop",
                    &format!(
                        "break 'round: LLM 返回 Stop 且无工具调用，无待处理消息 (round={}, text_len={})",
                        round_idx,
                        assistant_text.len()
                    ),
                );

                flush_streaming_content(
                    &streaming_content,
                    &streaming_reasoning_content,
                    &mut messages,
                    &display_messages,
                    &context_messages,
                    &mut assistant_reasoning,
                );

                metrics.total_llm_calls += 1;
                metrics.total_llm_elapsed_ms += call_start.elapsed().as_millis() as u64;
                break 'round;
            }

            #[allow(unreachable_code)]
            {
                break 'api_retry;
            }
        } // end 'api_retry
    } // end 'round

    write_info_log(
        "agent_loop",
        &format!(
            "agent loop 结束，发送 Done (共执行 {} 轮后退出 'round)",
            final_round_idx + 1
        ),
    );

    write_end_metrics(
        metrics,
        &session_id,
        context_tokens_peak,
        &sub_agent_metrics,
        &invoked_skills,
        &tx,
    );
}
