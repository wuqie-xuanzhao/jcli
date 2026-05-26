//! Hook metrics and entry structures

use std::collections::HashMap;
use std::sync::Mutex;

/// 单个 hook 的执行统计
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HookMetrics {
    /// 执行次数
    pub executions: u64,
    /// 成功次数
    pub successes: u64,
    /// 失败次数（含超时）
    pub failures: u64,
    /// 跳过次数（filter 不匹配）
    pub skipped: u64,
    /// 累计耗时（毫秒）
    pub total_duration_ms: u64,
}

/// 列出 hook 时的来源标记
pub const HOOK_SOURCE_BUILTIN: &str = "builtin";
pub const HOOK_SOURCE_USER: &str = "user";
pub const HOOK_SOURCE_PROJECT: &str = "project";
pub const HOOK_SOURCE_SESSION: &str = "session";

/// 列出 hook 时的摘要信息
pub struct HookEntry {
    /// Hook 目录名（目录布局下有值）
    pub name: Option<String>,
    pub event: crate::infra::hook::definition::HookEvent,
    pub source: &'static str,
    /// Hook 类型标签（bash / llm / builtin）
    pub hook_type: &'static str,
    /// Shell hook 的命令，LLM hook 的 prompt 摘要，或 Builtin hook 的名称
    pub label: String,
    /// Hook 的超时秒数
    pub timeout: Option<u64>,
    /// Hook 的失败策略
    pub on_error: Option<crate::infra::hook::definition::OnError>,
    /// Session hook 在该 event 下的局部索引（仅 session 来源有值，用于 remove 操作）
    pub session_index: Option<usize>,
    /// 条件过滤
    pub filter: Option<crate::infra::hook::definition::HookFilter>,
    /// 执行指标
    pub metrics: Option<HookMetrics>,
    /// Hook 唯一标识，格式：`builtin:<name>` / `user:<dir_name>` / `project:<dir_name>` / `session:<event_idx>`
    pub unique_id: String,
}
