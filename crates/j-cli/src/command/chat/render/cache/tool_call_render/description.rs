//! 工具描述提取模块
//!
//! 从各类工具的 arguments JSON 中提取简短描述信息，用于折叠模式显示

use crate::command::chat::constants::TOOL_ARG_PREVIEW_MAX_CHARS;
use crate::command::chat::tools::tool_names;

use super::truncate_str;

/// 从工具调用参数 JSON 中提取描述信息（用于折叠模式显示）
/// - Bash/Shell：提取 description 字段
/// - Read/Write/Edit/Glob/Grep：提取 path 或 file_path 字段
/// - Agent/Teammate：提取 description / role 字段
/// - Task：action + title
/// - TaskOutput：task_id
/// - WebSearch：query
/// - WebFetch：url
/// - Ask：question 文本
/// - TodoWrite/TodoRead：操作摘要
/// - Compact：focus
/// - EnterPlanMode：description
/// - LoadSkill：skill name
/// - RegisterHook：action + event
/// - SendMessage：to + message 预览
/// - EnterWorktree/ExitWorktree：name
/// - WorkDone：summary
/// - IgnoreMessage：静默标记
pub(crate) fn extract_tool_description_from_args(
    tool_name: &str,
    arguments: &str,
) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;

    match tool_name {
        // ── 文件操作 ──
        tool_names::SHELL => parsed.get("description")?.as_str().map(|s| s.to_string()),
        tool_names::READ
        | tool_names::WRITE
        | tool_names::EDIT
        | tool_names::GLOB
        | tool_names::GREP => parsed
            .get("path")
            .or_else(|| parsed.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),

        // ── Agent / Teammate ──
        tool_names::AGENT => parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_names::TEAMMATE => {
            let name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let role = parsed.get("role").and_then(|v| v.as_str()).unwrap_or(name);
            Some(role.to_string())
        }

        // ── Task（任务管理）──
        tool_names::TASK => {
            let action = parsed
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("task");
            let title = parsed.get("title").and_then(|v| v.as_str());
            match title {
                Some(t) => Some(format!("{}: {}", action, t)),
                None => Some(action.to_string()),
            }
        }

        // ── TaskOutput（后台任务输出）──
        tool_names::TASK_OUTPUT => {
            let task_id = parsed
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            Some(format!("获取任务 {} 输出", task_id))
        }

        // ── 网络 ──
        tool_names::WEB_SEARCH => parsed
            .get("query")
            .and_then(|v| v.as_str())
            .map(|s| format!("搜索: {}", s)),
        tool_names::WEB_FETCH => parsed
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_names::BROWSER => parsed
            .get("url")
            .or_else(|| parsed.get("action"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),

        // ── Ask（用户提问）──
        tool_names::ASK => {
            // questions 是数组，取第一个问题的 question 字段
            if let Some(questions) = parsed.get("questions").and_then(|v| v.as_array())
                && let Some(first) = questions.first()
            {
                return first
                    .get("question")
                    .and_then(|v| v.as_str())
                    .map(|s| truncate_str(s, TOOL_ARG_PREVIEW_MAX_CHARS));
            }
            None
        }

        // ── Todo ──
        tool_names::TODO_WRITE => {
            if let Some(todos) = parsed.get("todos").and_then(|v| v.as_array()) {
                let count = todos.len();
                Some(format!("更新 {} 项待办", count))
            } else {
                Some("更新待办".to_string())
            }
        }
        tool_names::TODO_READ => Some("读取待办列表".to_string()),

        // ── Compact（对话压缩）──
        tool_names::COMPACT => {
            let focus = parsed.get("focus").and_then(|v| v.as_str());
            match focus {
                Some(f) => Some(format!("压缩对话 (focus: {})", f)),
                None => Some("压缩对话".to_string()),
            }
        }

        // ── Plan ──
        tool_names::ENTER_PLAN_MODE => parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| format!("进入计划模式: {}", s))
            .or_else(|| Some("进入计划模式".to_string())),
        tool_names::EXIT_PLAN_MODE => Some("提交计划审批".to_string()),

        // ── LoadSkill ──
        tool_names::LOAD_SKILL => parsed
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| format!("加载技能: {}", s)),

        // ── RegisterHook ──
        tool_names::REGISTER_HOOK => {
            let action = parsed
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("register");
            let event = parsed.get("event").and_then(|v| v.as_str());
            match event {
                Some(e) => Some(format!("{} 钩子: {}", action, e)),
                None => Some(format!("{} 钩子", action)),
            }
        }

        // ── SendMessage ──
        tool_names::SEND_MESSAGE => {
            let to = parsed.get("to").and_then(|v| v.as_str());
            let msg = parsed
                .get("message")
                .and_then(|v| v.as_str())
                .map(|s| truncate_str(s, 40));
            match (to, msg) {
                (Some(t), Some(m)) => Some(format!("→ {} {}", t, m)),
                (Some(t), None) => Some(format!("→ {}", t)),
                (None, Some(m)) => Some(format!("广播: {}", m)),
                (None, None) => Some("发送消息".to_string()),
            }
        }

        // ── Worktree ──
        tool_names::ENTER_WORKTREE => parsed
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| format!("进入工作树: {}", s))
            .or_else(|| Some("进入工作树".to_string())),
        tool_names::EXIT_WORKTREE => Some("退出工作树".to_string()),

        // ── WorkDone ──
        tool_names::WORK_DONE => parsed
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| truncate_str(s, TOOL_ARG_PREVIEW_MAX_CHARS))
            .or_else(|| Some("工作完成".to_string())),

        // ── IgnoreMessage ──
        tool_names::IGNORE_MESSAGE => Some("忽略消息".to_string()),

        // ── ComputerUse ──
        #[cfg(target_os = "macos")]
        tool_names::COMPUTER_USE => parsed
            .get("action")
            .and_then(|v| v.as_str())
            .map(|s| format!("计算机操作: {}", s)),

        // ── LoadTool ──
        tool_names::LOAD_TOOL => parsed
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| format!("加载工具: {}", s)),

        // ── Session ──
        tool_names::SESSION => parsed
            .get("action")
            .and_then(|v| v.as_str())
            .map(|s| format!("会话: {}", s))
            .or_else(|| Some("会话操作".to_string())),

        _ => None,
    }
}
