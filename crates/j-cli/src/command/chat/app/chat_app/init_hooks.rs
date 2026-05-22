use crate::command::chat::constants::TODO_NAG_INTERVAL_ROUNDS;
use crate::command::chat::context::message_compress::{
    DEFAULT_OTHER_AGENT_TOOLCALL_THRESHOLD, compress_other_agent_toolcalls,
};
use crate::command::chat::infra::hook::{HookEvent, HookResult};
use crate::command::chat::storage::{ChatMessage, DisplayHint, MessageRole};
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::{BackgroundManager, build_running_summary};
use crate::command::chat::tools::task::{TaskManager, build_tasks_summary};
use crate::command::chat::tools::todo::TodoManager;
use std::sync::{Arc, Mutex};

/// 注册所有内置 hook（在 ChatApp::new 中调用）
pub(super) fn register_builtin_hooks(
    hook_manager: &Arc<Mutex<crate::command::chat::infra::hook::HookManager>>,
    task_manager: &Arc<TaskManager>,
    background_manager: &Arc<BackgroundManager>,
    tool_registry: &Arc<ToolRegistry>,
    teammate_manager: &Arc<Mutex<TeammateManager>>,
    todo_manager: &Arc<TodoManager>,
) {
    if let Ok(mut manager) = hook_manager.lock() {
        // 内置 hook 1: tasks_status — 替换 system_prompt 中的 {{.tasks}} 占位符
        let tasks_tm = Arc::clone(task_manager);
        manager.register_builtin(HookEvent::PreLlmRequest, "tasks_status", move |ctx| {
            let summary = build_tasks_summary(&tasks_tm);
            if let Some(ref prompt) = ctx.system_prompt
                && prompt.contains("{{.tasks}}")
            {
                return Some(HookResult {
                    system_prompt: Some(prompt.replace("{{.tasks}}", &summary)),
                    ..Default::default()
                });
            }
            None
        });

        // 内置 hook 2: background_status — 替换 {{.background_tasks}} 占位符 + 注入完成通知
        let bg_mgr = Arc::clone(background_manager);
        manager.register_builtin(
            HookEvent::PreLlmRequest,
            "background_status",
            move |ctx| {
                // ★ 先清理已死进程，确保状态准确
                bg_mgr.cleanup_dead_tasks();

                let running_summary = build_running_summary(&bg_mgr);
                let notifications = bg_mgr.drain_notifications();

                let mut result = HookResult::default();

                // 替换运行中任务占位符
                if let Some(ref prompt) = ctx.system_prompt
                    && prompt.contains("{{.background_tasks}}")
                {
                    result.system_prompt =
                        Some(prompt.replace("{{.background_tasks}}", &running_summary));
                }

                // 注入完成通知为 inject_messages
                if !notifications.is_empty() {
                    let mut inject = Vec::new();
                    for notif in notifications {
                        let body = format!(
                            "<background_task_completed>\n<task_id>{}</task_id>\n<command>{}</command>\n<status>{}</status>\n<result>\n{}\n</result>\n</background_task_completed>",
                            notif.task_id, notif.command, notif.status, notif.result
                        );
                        inject.push(ChatMessage {
                            role: MessageRole::User,
                            content: format!("<system-reminder>\n{}\n</system-reminder>", body),
                            tool_calls: None,
                            tool_call_id: None,
                            images: None,
                            reasoning_content: None,
                            sender_name: None,
                            recipient_name: None,
                            display_hint: DisplayHint::Normal,
                        });
                    }
                    result.inject_messages = Some(inject);
                }

                if result.system_prompt.is_some() || result.inject_messages.is_some() {
                    Some(result)
                } else {
                    None
                }
            },
        );

        // 内置 hook 3: session_state — 替换 {{.session_state}} 占位符
        let session_tr = Arc::clone(tool_registry);
        manager.register_builtin(HookEvent::PreLlmRequest, "session_state", move |ctx| {
            let summary = session_tr.build_session_state_summary();
            if let Some(ref prompt) = ctx.system_prompt
                && prompt.contains("{{.session_state}}")
            {
                return Some(HookResult {
                    system_prompt: Some(prompt.replace("{{.session_state}}", &summary)),
                    ..Default::default()
                });
            }
            None
        });

        // 内置 hook 4: teammates_status — 替换 {{.teammates}} 占位符
        let tm_mgr = Arc::clone(teammate_manager);
        manager.register_builtin(HookEvent::PreLlmRequest, "teammates_status", move |ctx| {
            let summary = tm_mgr.lock().map(|m| m.team_summary()).unwrap_or_default();
            if let Some(ref prompt) = ctx.system_prompt
                && prompt.contains("{{.teammates}}")
            {
                return Some(HookResult {
                    system_prompt: Some(prompt.replace("{{.teammates}}", &summary)),
                    ..Default::default()
                });
            }
            None
        });

        // 内置 hook 5: todo_nag — 当 todo 列表活跃但长时间未更新时注入提醒
        let todo_mgr = Arc::clone(todo_manager);
        manager.register_builtin(
            HookEvent::PreLlmRequest,
            "todo_nag",
            move |_ctx| {
                if !todo_mgr.has_todos() {
                    return None;
                }
                let turns = todo_mgr.turns_since_last_call();
                if turns < TODO_NAG_INTERVAL_ROUNDS {
                    return None;
                }
                let todos_summary = todo_mgr.format_todos_summary();
                let body = format!(
                    "<todo_reminder>\nYou have an active todo list but haven't updated it in 15+ rounds. Update it if progress has been made, or ignore this reminder if you are currently working on an item.\n<todos>\n{}\n</todos>\n</todo_reminder>",
                    todos_summary
                );
                let inject = vec![ChatMessage {
                    role: MessageRole::User,
                    content: format!("<system-reminder>\n{}\n</system-reminder>", body),
                    tool_calls: None,
                    tool_call_id: None,
                    images: None,
                    reasoning_content: None,
                    sender_name: None,
                    recipient_name: None,
                    display_hint: DisplayHint::Normal,
                }];
                Some(HookResult {
                    inject_messages: Some(inject),
                    ..Default::default()
                })
            },
        );

        // 内置 hook 6: broadcast_compress — 折叠来自其他 agent 的 tool call 广播
        //
        // 注册在末位，确保它在所有其他 hook（含 inject_messages）之后执行，
        // 这样即便有 hook 追加了 <Name> [调用工具 X] 格式的消息也能被折叠。
        // self_agent_name 取自线程本地身份：Main 线程返回 "Main"，teammate 线程返回
        // 其 teammate 名，SubAgent 线程返回 sub_id（SubAgent 的 messages 里几乎不会有
        // 广播，折叠无副作用）。
        manager.register_builtin(HookEvent::PreLlmRequest, "broadcast_compress", |ctx| {
            let messages = ctx.messages.as_ref()?;
            let self_name = crate::command::chat::agent::thread_identity::current_agent_name();
            let compressed = compress_other_agent_toolcalls(
                messages,
                &self_name,
                DEFAULT_OTHER_AGENT_TOOLCALL_THRESHOLD,
            );
            if compressed.len() == messages.len() {
                return None;
            }
            Some(HookResult {
                messages: Some(compressed),
                ..Default::default()
            })
        });
    }
}
