use super::meta::{SessionMetaFile, load_session_meta_file, save_session_meta_file};
use super::paths::{SessionPaths, sessions_dir};
use crate::constants::MESSAGE_PREVIEW_MAX_LEN;
use crate::storage::types::{ChatMessage, MessageRole, SessionEvent, SessionOp};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 查找最近修改的 session ID（用于 --continue）
pub fn find_latest_session_id() -> Option<String> {
    let dir = sessions_dir();
    let mut entries: Vec<(std::time::SystemTime, String)> = Vec::new();
    let read_dir = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return None,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let transcript = path.join("transcript.jsonl");
        if let Ok(meta) = transcript.metadata()
            && let Ok(modified) = meta.modified()
        {
            entries.push((modified, id.to_string()));
        }
    }
    entries.sort_by_key(|b| std::cmp::Reverse(b.0));
    entries.into_iter().next().map(|(_, id)| id)
}

/// 追加一个事件到 session JSONL 文件（append-only，POSIX 下原子安全）
///
/// 同时增量更新 `session.json` 元数据。
pub fn append_session_event(session_id: &str, event: &SessionEvent) -> bool {
    let paths = SessionPaths::new(session_id);
    if paths.ensure_dir().is_err() {
        return false;
    }
    let path = paths.transcript();
    let ok = match serde_json::to_string(event) {
        Ok(line) => match fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => writeln!(file, "{}", line).is_ok(),
            Err(_) => false,
        },
        Err(_) => false,
    };
    if ok {
        update_session_meta_on_event(session_id, event);
    }
    ok
}

/// 追加一条操作审计记录到 ops.jsonl（append-only，与 append_session_event 同模式）
pub fn append_session_op(session_id: &str, op: &SessionOp) -> bool {
    let paths = SessionPaths::new(session_id);
    if paths.ensure_dir().is_err() {
        return false;
    }
    let path = paths.ops_file();
    match serde_json::to_string(op) {
        Ok(line) => match fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => writeln!(file, "{}", line).is_ok(),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// 读取 session 的所有操作审计记录
#[allow(dead_code)]
pub fn load_session_ops(session_id: &str) -> Vec<SessionOp> {
    let path = SessionPaths::new(session_id).ops_file();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut ops = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(op) = serde_json::from_str::<SessionOp>(line) {
            ops.push(op);
        }
    }
    ops
}

/// 增量更新 session.json 元数据（追加事件后调用）
fn update_session_meta_on_event(session_id: &str, event: &SessionEvent) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut meta = load_session_meta_file(session_id).unwrap_or_else(|| SessionMetaFile {
        id: session_id.to_string(),
        title: String::new(),
        message_count: 0,
        created_at: now,
        updated_at: now,
        model: None,
        auto_approve: false,
    });
    meta.updated_at = now;
    match event {
        SessionEvent::Msg { message: msg, .. } => {
            meta.message_count += 1;
            if meta.title.is_empty() && msg.role == MessageRole::User && !msg.content.is_empty() {
                meta.title = msg.content.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect();
            }
        }
        SessionEvent::Clear => {
            meta.message_count = 0;
        }
        SessionEvent::Restore { messages } => {
            meta.message_count = messages.len();
            if meta.title.is_empty()
                && let Some(first_user) = messages
                    .iter()
                    .find(|m| m.role == MessageRole::User && !m.content.is_empty())
            {
                meta.title = first_user
                    .content
                    .chars()
                    .take(MESSAGE_PREVIEW_MAX_LEN)
                    .collect();
            }
        }
        SessionEvent::Metrics { .. } => {}
    }
    let _ = save_session_meta_file(&meta);
}

/// 从 JSONL 文件 replay 出消息列表（供 resume 等功能使用）
pub fn load_session(session_id: &str) -> Vec<ChatMessage> {
    let path = SessionPaths::new(session_id).transcript();
    if !path.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut messages: Vec<ChatMessage> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<SessionEvent>(line) {
            Ok(event) => match event {
                SessionEvent::Msg { message, .. } => messages.push(message),
                SessionEvent::Clear => messages.clear(),
                SessionEvent::Restore { messages: restored } => messages = restored,
                SessionEvent::Metrics { .. } => {}
            },
            Err(_) => {
                // 损坏行直接跳过，继续处理剩余行
            }
        }
    }

    // ★ 存量清理：移除历史遗留的孤立 tool_call / tool_result。
    //   检测到变化时追加一条 Restore 事件，让 jsonl 下次加载直接从干净快照出发，
    //   orphan 不再反复出现在 sanitize 日志里。
    if let Some(sanitized) = sanitize_loaded_messages(&messages) {
        let restore_event = SessionEvent::Restore {
            messages: sanitized.clone(),
        };
        append_session_event(session_id, &restore_event);
        messages = sanitized;
    }

    messages
}

/// 从 display.jsonl replay 出 display 消息列表。
///
/// 逻辑同 `load_session`，但只返回 `Vec<ChatMessage>`，
/// 也不做 sanitize（display 消息无需配对校验）。
pub fn load_display_session(session_id: &str) -> Vec<ChatMessage> {
    let path = SessionPaths::new(session_id).display();
    if !path.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut messages: Vec<ChatMessage> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<SessionEvent>(line) {
            match event {
                SessionEvent::Msg { message, .. } => messages.push(message),
                SessionEvent::Clear => messages.clear(),
                SessionEvent::Restore { messages: restored } => messages = restored,
                SessionEvent::Metrics { .. } => {}
            }
        }
    }
    messages
}

/// 双向配对清理：
///   - 移除 tool_call_id 为空或在任何 assistant tool_calls 中找不到对应项的 tool result
///   - 移除 assistant tool_calls 中 id 为空或找不到对应 tool result 的条目；
///     tool_calls 被全部清空时置为 None（保留 content 文本）
///
/// 返回 `Some(sanitized)` 表示发生了变化，`None` 表示原样可用。
fn sanitize_loaded_messages(messages: &[ChatMessage]) -> Option<Vec<ChatMessage>> {
    let tool_result_ids: std::collections::HashSet<String> = messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
        .filter_map(|m| m.tool_call_id.clone())
        .filter(|id| !id.is_empty())
        .collect();

    let assistant_tool_call_ids: std::collections::HashSet<String> = messages
        .iter()
        .filter(|m| m.role == MessageRole::Assistant)
        .flat_map(|m| m.tool_calls.as_deref().unwrap_or(&[]))
        .map(|tc| tc.id.clone())
        .filter(|id| !id.is_empty())
        .collect();

    let mut changed = false;
    let mut out: Vec<ChatMessage> = Vec::with_capacity(messages.len());
    for msg in messages {
        if msg.role == MessageRole::Tool {
            let id = msg.tool_call_id.as_deref().unwrap_or("");
            if id.is_empty() || !assistant_tool_call_ids.contains(id) {
                changed = true;
                continue;
            }
            out.push(msg.clone());
        } else if msg.role == MessageRole::Assistant {
            if let Some(ref tcs) = msg.tool_calls {
                let kept: Vec<_> = tcs
                    .iter()
                    .filter(|tc| !tc.id.is_empty() && tool_result_ids.contains(&tc.id))
                    .cloned()
                    .collect();
                if kept.len() != tcs.len() {
                    changed = true;
                    let mut new_msg = msg.clone();
                    new_msg.tool_calls = if kept.is_empty() { None } else { Some(kept) };
                    // 若 tool_calls 被清空且没有文本内容，整条消息也无意义——跳过
                    if new_msg.tool_calls.is_none() && new_msg.content.trim().is_empty() {
                        continue;
                    }
                    out.push(new_msg);
                } else {
                    out.push(msg.clone());
                }
            } else {
                out.push(msg.clone());
            }
        } else {
            out.push(msg.clone());
        }
    }

    if changed { Some(out) } else { None }
}

/// 从 JSONL 文件按出现顺序读取 `(ChatMessage, timestamp_ms)` 列表。
///
/// 供 teammate / subagent 等独立 transcript 的读取使用：保留时间戳、不做 Clear/Restore 处理。
#[allow(dead_code)]
pub fn read_transcript_with_timestamps(path: &Path) -> Vec<(ChatMessage, u64)> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<(ChatMessage, u64)> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(SessionEvent::Msg {
            message,
            timestamp_ms,
        }) = serde_json::from_str::<SessionEvent>(line)
        {
            out.push((message, timestamp_ms));
        }
    }
    out
}

/// 向任意路径的 JSONL 文件 append 一条事件（append-only；用于 teammate/subagent 独立 transcript）。
pub fn append_event_to_path(path: &Path, event: &SessionEvent) -> bool {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = match serde_json::to_string(event) {
        Ok(s) => s,
        Err(_) => return false,
    };
    match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => writeln!(file, "{}", line).is_ok(),
        Err(_) => false,
    }
}
