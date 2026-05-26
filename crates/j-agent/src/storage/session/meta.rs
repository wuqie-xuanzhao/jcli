use super::paths::{SessionPaths, sessions_dir};
use crate::constants::MESSAGE_PREVIEW_MAX_LEN;
use crate::storage::types::{MessageRole, SessionEvent, SessionMetrics};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// session.json 元数据文件内容（持久化到 `sessions/<id>/session.json`）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetaFile {
    /// 会话 ID
    pub id: String,
    /// 会话标题（首条 user 消息截断）
    #[serde(default)]
    pub title: String,
    /// 消息计数
    pub message_count: usize,
    /// 创建时间戳（epoch seconds）
    pub created_at: u64,
    /// 最后更新时间戳（epoch seconds）
    pub updated_at: u64,
    /// 使用的模型名称
    #[serde(default)]
    pub model: Option<String>,
    /// 是否自动批准所有操作（bypass 模式）
    #[serde(default)]
    pub auto_approve: bool,
}

/// 会话元数据（用于会话列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    /// 会话标题（从 session.json 读取）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub message_count: usize,
    pub first_message_preview: Option<String>,
    pub updated_at: u64,
}

/// 加载 session.json 元数据（不存在返回 None）
pub fn load_session_meta_file(session_id: &str) -> Option<SessionMetaFile> {
    let path = SessionPaths::new(session_id).meta_file();
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 保存 session.json 元数据
pub fn save_session_meta_file(meta: &SessionMetaFile) -> bool {
    let paths = SessionPaths::new(&meta.id);
    if paths.ensure_dir().is_err() {
        return false;
    }
    match serde_json::to_string_pretty(meta) {
        Ok(json) => fs::write(paths.meta_file(), json).is_ok(),
        Err(_) => false,
    }
}

/// 从 transcript.jsonl 逐行扫描生成元数据（懒生成 / 迁移用）
fn derive_session_meta_from_transcript(session_id: &str) -> Option<SessionMetaFile> {
    let paths = SessionPaths::new(session_id);
    let transcript = paths.transcript();
    let content = fs::read_to_string(&transcript).ok()?;

    let mut message_count: usize = 0;
    let mut first_user_preview: Option<String> = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<SessionEvent>(line) {
            match event {
                SessionEvent::Msg {
                    message: ref msg, ..
                } => {
                    message_count += 1;
                    if first_user_preview.is_none()
                        && msg.role == MessageRole::User
                        && !msg.content.is_empty()
                    {
                        first_user_preview =
                            Some(msg.content.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect());
                    }
                }
                SessionEvent::Clear => {
                    message_count = 0;
                    first_user_preview = None;
                }
                SessionEvent::Restore { ref messages } => {
                    message_count = messages.len();
                    first_user_preview = messages
                        .iter()
                        .find(|m| m.role == MessageRole::User && !m.content.is_empty())
                        .map(|m| m.content.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect());
                }
                SessionEvent::Metrics { .. } => {}
            }
        }
    }

    let updated_at = transcript
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Some(SessionMetaFile {
        id: session_id.to_string(),
        title: first_user_preview.clone().unwrap_or_default(),
        message_count,
        created_at: updated_at,
        updated_at,
        model: None,
        auto_approve: false,
    })
}

/// 列出所有会话的元数据，按更新时间倒序
///
/// 优先读 `session.json` 元数据文件（O(1)），不存在时 fallback 到逐行扫描
/// `transcript.jsonl` 并懒生成 `session.json`。
pub fn list_sessions() -> Vec<SessionMeta> {
    let dir = sessions_dir();
    let read_dir = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut ids: Vec<String> = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if path.join("transcript.jsonl").exists() {
            ids.push(id.to_string());
        }
    }

    let mut sessions: Vec<SessionMeta> = Vec::with_capacity(ids.len());
    for id in ids {
        // 优先读 session.json
        if let Some(meta_file) = load_session_meta_file(&id) {
            sessions.push(SessionMeta {
                id: meta_file.id,
                title: if meta_file.title.is_empty() {
                    None
                } else {
                    Some(meta_file.title)
                },
                message_count: meta_file.message_count,
                first_message_preview: None,
                updated_at: meta_file.updated_at,
            });
            continue;
        }

        // fallback：逐行扫描 transcript 并懒生成 session.json
        if let Some(derived) = derive_session_meta_from_transcript(&id) {
            let title = if derived.title.is_empty() {
                None
            } else {
                Some(derived.title.clone())
            };
            let preview_for_ui = title.clone();
            let _ = save_session_meta_file(&derived);
            sessions.push(SessionMeta {
                id: derived.id,
                title,
                message_count: derived.message_count,
                first_message_preview: preview_for_ui,
                updated_at: derived.updated_at,
            });
        }
    }
    sessions.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
    sessions
}

/// 生成会话 ID（时间戳微秒 + 进程 ID，无需外部依赖）
pub fn generate_session_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let pid = std::process::id();
    format!("{:x}-{:x}", ts, pid)
}

/// 删除指定 session 目录
pub fn delete_session(session_id: &str) -> bool {
    let paths = SessionPaths::new(session_id);
    let dir = paths.dir().to_path_buf();
    if dir.exists()
        && let Err(e) = fs::remove_dir_all(&dir)
    {
        eprintln!("[ERROR] ✖️ 删除 session 目录失败: {}", e);
        return false;
    }
    true
}

/// 将 SessionMetrics 写入 sessions/<id>/metrics.json（覆盖写，JSON pretty）
pub fn write_session_metrics(session_id: &str, metrics: &SessionMetrics) -> bool {
    let paths = SessionPaths::new(session_id);
    let path = paths.metrics_file();
    match serde_json::to_string_pretty(metrics) {
        Ok(json) => fs::write(&path, json).is_ok(),
        Err(_) => false,
    }
}
