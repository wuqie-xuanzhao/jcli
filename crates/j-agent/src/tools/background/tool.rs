use super::manager::BackgroundManager;
use crate::constants::{BG_TASK_DEFAULT_TIMEOUT_MS, BG_TASK_MAX_TIMEOUT_MS};
use crate::tools::{PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

/// TaskOutputTool 参数
#[derive(Deserialize, JsonSchema)]
struct TaskOutputParams {
    /// The task ID to get output from (returned by Shell with run_in_background: true)
    task_id: String,
    /// Whether to wait for task completion (default: true). Set to false for a non-blocking check of current status.
    #[serde(default = "default_block")]
    block: bool,
    /// Max wait time in milliseconds when block=true (default: 30000, max: 600000)
    #[serde(default = "default_timeout_ms")]
    timeout: u64,
}

fn default_block() -> bool {
    true
}

fn default_timeout_ms() -> u64 {
    BG_TASK_DEFAULT_TIMEOUT_MS
}

/// 查询后台任务输出的工具（替代 CheckBackgroundTool）
#[derive(Debug)]
pub struct TaskOutputTool {
    pub manager: Arc<BackgroundManager>,
}

impl TaskOutputTool {
    pub const NAME: &'static str = "TaskOutput";
}

impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        r#"
        Retrieves output from a running or completed background task (started via Shell with run_in_background: true).
        Use block=true (default) to wait for task completion; use block=false for a non-blocking status check.
        Returns the task output along with status information.
        "#.into()
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<TaskOutputParams>()
    }

    #[allow(clippy::too_many_lines)]
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: TaskOutputParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let timeout_ms = params.timeout.min(BG_TASK_MAX_TIMEOUT_MS);

        // 若任务不存在，直接报错
        if self.manager.get_task_status(&params.task_id).is_none() {
            return ToolResult {
                output: format!("后台任务 {} 不存在", params.task_id),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // block=true 且任务仍在运行时，轮询等待
        if params.block && self.manager.is_running(&params.task_id) {
            let start = Instant::now();
            let timeout = Duration::from_millis(timeout_ms);
            loop {
                if !self.manager.is_running(&params.task_id) {
                    break;
                }
                // ★ 检查取消信号：用户取消请求时应立即中断等待
                if cancelled.load(Ordering::Relaxed) {
                    let info = self
                        .manager
                        .get_task_status(&params.task_id)
                        .unwrap_or(json!({}));
                    let mut obj = info.clone();
                    if let Some(map) = obj.as_object_mut() {
                        map.insert(
                            "note".to_string(),
                            json!("cancelled: request was cancelled while waiting for task output"),
                        );
                    }
                    return ToolResult {
                        output: serde_json::to_string_pretty(&obj).unwrap_or_default(),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
                if start.elapsed() >= timeout {
                    // 超时，返回当前状态
                    let info = self
                        .manager
                        .get_task_status(&params.task_id)
                        .unwrap_or(json!({}));
                    let mut obj = info.clone();
                    if let Some(map) = obj.as_object_mut() {
                        map.insert(
                            "note".to_string(),
                            json!("still running: timeout waiting for completion"),
                        );
                    }
                    return ToolResult {
                        output: serde_json::to_string_pretty(&obj).unwrap_or_default(),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        // 返回当前状态（已完成或 block=false）
        match self.manager.get_task_status(&params.task_id) {
            Some(info) => ToolResult {
                output: serde_json::to_string_pretty(&info).unwrap_or_default(),
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
            None => ToolResult {
                output: format!("后台任务 {} 不存在", params.task_id),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }
}
