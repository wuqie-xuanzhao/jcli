use std::sync::{Arc, Mutex};

use crate::agent::tool_processor::push_both;
use crate::context::compact::CompactResult;
use crate::storage::{ChatMessage, DisplayHint, MessageRole, ToolCallItem};

/// 流式响应中逐步聚合的工具调用片段（按 chunk index 聚合 id/name/arguments）
pub(super) struct StreamingToolCallPart {
    pub(super) call_id: String,
    pub(super) function_name: String,
    pub(super) function_arguments: String,
}

/// auto_compact 成功后，向 messages 和双通道注入 Compact 工具调用 + 结果消息，
/// 等同于 LLM 手动调用 CompactTool 的效果。
///
/// UI 显示顺序（从上到下）：
/// 1. recent_user_messages（用户最近的消息）
/// 2. assistant tool_call (Compact)
/// 3. tool result（压缩摘要）
///
/// LLM 上下文顺序（messages）：
/// 1. recent_user_messages
/// 2. assistant tool_call
/// 3. tool result
pub(super) fn push_compact_tool_messages(
    messages: &mut Vec<ChatMessage>,
    display: &Arc<Mutex<Vec<ChatMessage>>>,
    context: &Arc<Mutex<Vec<ChatMessage>>>,
    compact_result: &CompactResult,
) {
    let tool_call_id = format!("compact_auto_{}", compact_result.messages_before);

    // 1. 先推送 recent_user_messages（UI 中用户消息在 compact 摘要上方）
    //    这些消息已在 messages 中（由 auto_compact 添加），只需同步到双通道
    for msg in &compact_result.recent_user_messages {
        push_both(display, context, msg.clone());
    }

    // 2. 创建 Compact 工具调用消息（模拟 LLM 调用 Compact 工具）
    let tool_call_item = ToolCallItem {
        id: tool_call_id.clone(),
        name: "Compact".to_string(),
        arguments: r#"{"reason":"auto_compact"}"#.to_string(),
    };
    let tool_call_msg = ChatMessage {
        role: MessageRole::Assistant,
        content: String::new(),
        tool_calls: Some(vec![tool_call_item]),
        tool_call_id: None,
        images: None,
        reasoning_content: None,
        sender_name: None,
        recipient_name: None,
        display_hint: DisplayHint::Normal,
    };
    messages.push(tool_call_msg.clone());
    push_both(display, context, tool_call_msg);

    // 3. tool result 消息：包含摘要内容，UI 以边框形式展示
    let result_content = format!(
        "📦 上下文已压缩 ({} 条消息 → 摘要, transcript: {})\n\n{}",
        compact_result.messages_before, compact_result.transcript_path, compact_result.summary,
    );
    let tool_msg = ChatMessage {
        role: MessageRole::Tool,
        content: result_content,
        tool_calls: None,
        tool_call_id: Some(tool_call_id),
        images: None,
        reasoning_content: None,
        sender_name: None,
        recipient_name: None,
        display_hint: DisplayHint::Normal,
    };
    messages.push(tool_msg.clone());
    push_both(display, context, tool_msg);
}
