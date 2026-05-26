use super::super::config::agent_data_dir;
use std::fs;
use std::path::{Path, PathBuf};

/// 获取 sessions 目录: ~/.jdata/agent/data/sessions/
pub fn sessions_dir() -> PathBuf {
    let dir = agent_data_dir().join("sessions");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取单个 session 的 JSONL 文件路径（兼容别名，指向新布局主文件）
pub fn session_file_path(session_id: &str) -> PathBuf {
    SessionPaths::new(session_id).transcript()
}

/// Session 目录布局抽象。
///
/// 布局：`sessions/<id>/transcript.jsonl`。
#[derive(Debug)]
pub struct SessionPaths {
    dir: PathBuf,
}

impl SessionPaths {
    /// 根据 session ID 创建 SessionPaths
    pub fn new(session_id: &str) -> Self {
        let dir = sessions_dir().join(session_id);
        Self { dir }
    }

    /// 获取会话目录路径
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 主数据文件：`sessions/<id>/transcript.jsonl`
    pub fn transcript(&self) -> PathBuf {
        self.dir.join("transcript.jsonl")
    }

    /// 元数据文件：`sessions/<id>/session.json`
    pub fn meta_file(&self) -> PathBuf {
        self.dir.join("session.json")
    }

    /// compact 快照目录：`sessions/<id>/.transcripts/`
    pub fn transcripts_dir(&self) -> PathBuf {
        self.dir.join(".transcripts")
    }

    /// Teammate 状态文件：`sessions/<id>/teammates.json`
    pub fn teammates_file(&self) -> PathBuf {
        self.dir.join("teammates.json")
    }

    /// Display 消息 JSONL：`sessions/<id>/display.jsonl`
    pub fn display(&self) -> PathBuf {
        self.dir.join("display.jsonl")
    }

    /// Teammate 独立目录根：`sessions/<id>/teammates/`
    pub fn teammates_dir(&self) -> PathBuf {
        self.dir.join("teammates")
    }

    /// 单个 teammate 的独立子目录：`sessions/<id>/teammates/<sanitized_name>/`
    pub fn teammate_dir(&self, sanitized_name: &str) -> PathBuf {
        self.teammates_dir().join(sanitized_name)
    }

    /// 单个 teammate 的 transcript JSONL 路径：`sessions/<id>/teammates/<sanitized_name>/transcript.jsonl`
    pub fn teammate_transcript(&self, sanitized_name: &str) -> PathBuf {
        self.teammate_dir(sanitized_name).join("transcript.jsonl")
    }

    /// 单个 teammate 的 todo 文件路径：`sessions/<id>/teammates/<sanitized_name>/todos.json`
    pub fn teammate_todos_file(&self, sanitized_name: &str) -> PathBuf {
        self.teammate_dir(sanitized_name).join("todos.json")
    }

    /// SubAgent 状态文件：`sessions/<id>/subagents.json`
    pub fn subagents_file(&self) -> PathBuf {
        self.dir.join("subagents.json")
    }

    /// SubAgent 独立目录根：`sessions/<id>/subagents/`
    pub fn subagents_dir(&self) -> PathBuf {
        self.dir.join("subagents")
    }

    /// 单个 subagent 的独立子目录：`sessions/<id>/subagents/<sub_id>/`
    pub fn subagent_dir(&self, sub_id: &str) -> PathBuf {
        self.subagents_dir().join(sub_id)
    }

    /// 单个 subagent 的 transcript JSONL 路径：`sessions/<id>/subagents/<sub_id>/transcript.jsonl`
    pub fn subagent_transcript(&self, sub_id: &str) -> PathBuf {
        self.subagent_dir(sub_id).join("transcript.jsonl")
    }

    /// 单个 subagent 的 todo 文件路径：`sessions/<id>/subagents/<sub_id>/todos.json`
    pub fn subagent_todos_file(&self, sub_id: &str) -> PathBuf {
        self.subagent_dir(sub_id).join("todos.json")
    }

    /// Task 状态文件：`sessions/<id>/tasks.json`
    pub fn tasks_file(&self) -> PathBuf {
        self.dir.join("tasks.json")
    }

    /// Todo 状态文件：`sessions/<id>/todos.json`
    pub fn todos_file(&self) -> PathBuf {
        self.dir.join("todos.json")
    }

    /// Plan 状态文件：`sessions/<id>/plan.json`
    pub fn plan_file(&self) -> PathBuf {
        self.dir.join("plan.json")
    }

    /// InvokedSkills 状态文件：`sessions/<id>/skills.json`
    pub fn skills_file(&self) -> PathBuf {
        self.dir.join("skills.json")
    }

    /// Session Hook 状态文件：`sessions/<id>/hooks.json`
    pub fn hooks_file(&self) -> PathBuf {
        self.dir.join("hooks.json")
    }

    /// Sandbox 状态文件：`sessions/<id>/sandbox.json`
    pub fn sandbox_file(&self) -> PathBuf {
        self.dir.join("sandbox.json")
    }

    /// LoadTool 已加载的 deferred 工具：`sessions/<id>/loaded_deferred.json`
    pub fn loaded_deferred_file(&self) -> PathBuf {
        self.dir.join("loaded_deferred.json")
    }

    /// 操作审计文件：sessions/<id>/ops.jsonl
    pub fn ops_file(&self) -> PathBuf {
        self.dir.join("ops.jsonl")
    }

    /// 性能指标文件：sessions/<id>/metrics.json
    pub fn metrics_file(&self) -> PathBuf {
        self.dir.join("metrics.json")
    }

    /// 确保会话目录存在
    pub fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)
    }

    /// 返回 session ID（即目录名）
    #[allow(dead_code)]
    pub fn id(&self) -> &str {
        self.dir.file_name().and_then(|s| s.to_str()).unwrap_or("")
    }
}
