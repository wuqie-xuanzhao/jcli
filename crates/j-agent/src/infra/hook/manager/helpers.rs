//! Hook helper functions for label extraction, filtering, and error handling

use crate::infra::hook::definition::*;
use crate::infra::hook::types::*;
use crate::infra::hook::manager::metrics::{HookMetrics, HOOK_SOURCE_BUILTIN, HOOK_SOURCE_USER, HOOK_SOURCE_PROJECT, HOOK_SOURCE_SESSION};

/// Hook 引用（附带来源标记）
pub(super) struct HookRef<'a> {
    pub(super) kind: &'a HookKind,
    pub(super) source: &'static str,
    pub(super) session_index: Option<usize>,
}

/// 收集指定事件的所有 hook（内置→用户→项目→session）
pub(super) fn collect_hooks_for_event(manager: &HookManager, event: HookEvent) -> Vec<HookRef<'_>> {
    let mut all_hooks: Vec<HookRef<'_>> = Vec::new();

    if let Some(hooks) = manager.builtin_hooks().get(&event) {
        for h in hooks.iter() {
            all_hooks.push(HookRef {
                kind: h,
                source: HOOK_SOURCE_BUILTIN,
                session_index: None,
            });
        }
    }
    if let Some(hooks) = manager.user_hooks().get(&event) {
        for h in hooks.iter() {
            all_hooks.push(HookRef {
                kind: h,
                source: HOOK_SOURCE_USER,
                session_index: None,
            });
        }
    }
    if let Some(hooks) = manager.project_hooks().get(&event) {
        for h in hooks.iter() {
            all_hooks.push(HookRef {
                kind: h,
                source: HOOK_SOURCE_PROJECT,
                session_index: None,
            });
        }
    }
    if let Some(hooks) = manager.session_hooks().get(&event) {
        for (idx, h) in hooks.iter().enumerate() {
            all_hooks.push(HookRef {
                kind: h,
                source: HOOK_SOURCE_SESSION,
                session_index: Some(idx),
            });
        }
    }

    all_hooks
}

/// 将 hook 执行结果合并到 context（链式传递）和 final_result（最终返回）
pub(super) fn merge_hook_result_into(
    hook_result: &HookResult,
    context: &mut HookContext,
    final_result: &mut HookResult,
) {
    if let Some(ref msgs) = hook_result.messages {
        context.messages = Some(msgs.clone());
        final_result.messages = context.messages.clone();
    }
    if let Some(ref sp) = hook_result.system_prompt {
        context.system_prompt = Some(sp.clone());
        final_result.system_prompt = context.system_prompt.clone();
    }
    if let Some(ref ui) = hook_result.user_input {
        context.user_input = Some(ui.clone());
        final_result.user_input = context.user_input.clone();
    }
    if let Some(ref ao) = hook_result.assistant_output {
        context.assistant_output = Some(ao.clone());
        final_result.assistant_output = context.assistant_output.clone();
    }
    if let Some(ref ta) = hook_result.tool_arguments {
        context.tool_arguments = Some(ta.clone());
        final_result.tool_arguments = context.tool_arguments.clone();
    }
    if let Some(ref tr) = hook_result.tool_result {
        context.tool_result = Some(tr.clone());
        final_result.tool_result = context.tool_result.clone();
    }
    if let Some(ref inject) = hook_result.inject_messages {
        let existing = final_result.inject_messages.get_or_insert_with(Vec::new);
        existing.extend(inject.clone());
    }
    if let Some(ref rf) = hook_result.retry_feedback {
        final_result.retry_feedback = Some(rf.clone());
    }
    if let Some(ref ac) = hook_result.additional_context {
        final_result.additional_context = Some(ac.clone());
    }
    if let Some(ref sm) = hook_result.system_message {
        final_result.system_message = Some(sm.clone());
    }
    if let Some(ref te) = hook_result.tool_error {
        final_result.tool_error = Some(te.clone());
    }
}

/// 处理 hook 执行失败：按 on_error 策略返回 Stop 或 None（Skip，继续执行下一个）
/// 返回 `Some(HookResult)` 表示应中止链，返回 `None` 表示跳过继续
pub(super) fn handle_hook_error(kind: &HookKind, _label: &str) -> Option<HookResult> {
    match hook_on_error_strategy(kind) {
        OnError::Stop => Some(HookResult {
            action: Some(HookAction::Stop),
            ..Default::default()
        }),
        OnError::Skip => None,
    }
}

/// 生成 hook 唯一标识，格式：`source:unique_key`
pub fn hook_unique_id(source: &str, kind: &HookKind, session_index: Option<usize>) -> String {
    let key = match kind {
        HookKind::Builtin(b) => b.name.clone(),
        HookKind::Shell(s) => s
            .name
            .clone()
            .unwrap_or_else(|| s.command.chars().take(40).collect()),
        HookKind::Llm(l) => l
            .name
            .clone()
            .unwrap_or_else(|| l.prompt.chars().take(40).collect()),
    };
    match session_index {
        Some(idx) => format!("{}:{}", source, idx),
        None => format!("{}:{}", source, key),
    }
}

/// 获取 hook 的名称（目录布局下的目录名）
pub(crate) fn hook_name(kind: &HookKind) -> Option<&str> {
    match kind {
        HookKind::Shell(shell) => shell.name.as_deref(),
        HookKind::Llm(llm) => llm.name.as_deref(),
        HookKind::Builtin(builtin) => Some(&builtin.name),
    }
}

/// 获取 hook 的显示标签（Shell 用命令，LLM用 prompt 摘要，Builtin 用名称）
pub(crate) fn hook_label(kind: &HookKind) -> String {
    match kind {
        HookKind::Shell(shell) => {
            if let Some(ref name) = shell.name {
                format!("{}: {}", name, shell.command)
            } else {
                shell.command.clone()
            }
        }
        HookKind::Llm(llm) => {
            // 取 prompt 前一行或前 80 字符作为标签
            let first_line = llm
                .prompt
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or(&llm.prompt);
            let prompt_preview = if first_line.len() > crate::constants::HOOK_PROMPT_PREVIEW_MAX_LEN
            {
                format!(
                    "{}...",
                    &first_line[..crate::constants::HOOK_PROMPT_PREVIEW_MAX_LEN]
                )
            } else {
                first_line.to_string()
            };
            if let Some(ref name) = llm.name {
                format!("[llm: {}] {}", name, prompt_preview)
            } else {
                format!("[llm: {}]", prompt_preview)
            }
        }
        HookKind::Builtin(builtin) => format!("[builtin: {}]", builtin.name),
    }
}

/// 获取 hook 类型字符串
pub(crate) fn hook_type_str(kind: &HookKind) -> &'static str {
    match kind {
        HookKind::Shell(_) => "bash",
        HookKind::Llm(_) => "llm",
        HookKind::Builtin(_) => "builtin",
    }
}

/// 获取 hook 的超时秒数
pub(crate) fn hook_timeout(kind: &HookKind) -> Option<u64> {
    match kind {
        HookKind::Shell(shell) => Some(shell.timeout),
        HookKind::Llm(llm) => Some(llm.timeout),
        HookKind::Builtin(_) => None,
    }
}

/// 获取 hook 的重试次数
pub(crate) fn hook_retry_count(kind: &HookKind) -> u32 {
    match kind {
        HookKind::Shell(shell) => shell.retry,
        HookKind::Llm(llm) => llm.retry,
        HookKind::Builtin(_) => 0,
    }
}

/// 获取 hook 的失败策略（用于 list 展示）
pub(crate) fn hook_on_error(kind: &HookKind) -> Option<OnError> {
    match kind {
        HookKind::Shell(shell) => Some(shell.on_error),
        HookKind::Llm(llm) => Some(llm.on_error),
        HookKind::Builtin(_) => None,
    }
}

/// 获取 hook 执行失败时的实际策略（Shell/LLM 按配置，Builtin 一律 Abort）
pub(crate) fn hook_on_error_strategy(kind: &HookKind) -> OnError {
    match kind {
        HookKind::Shell(shell) => shell.on_error,
        HookKind::Llm(llm) => llm.on_error,
        HookKind::Builtin(_) => OnError::Stop,
    }
}

/// 获取 hook 的条件过滤器
pub(crate) fn hook_filter(kind: &HookKind) -> Option<&HookFilter> {
    match kind {
        HookKind::Shell(shell) if !shell.filter.is_empty() => Some(&shell.filter),
        HookKind::Llm(llm) if !llm.filter.is_empty() => Some(&llm.filter),
        HookKind::Shell(_) | HookKind::Llm(_) | HookKind::Builtin(_) => None,
    }
}

/// 检查 hook 是否应在当前 context 下执行（无 filter 或 filter 匹配时返回 true）
pub(crate) fn hook_should_execute(kind: &HookKind, context: &HookContext) -> bool {
    match kind {
        HookKind::Shell(shell) => shell.filter.matches(context),
        HookKind::Llm(llm) => llm.filter.matches(context),
        HookKind::Builtin(_) => true,
    }
}

/// Make entry helper for list_hooks
pub(super) fn make_hook_entry(
    event: HookEvent,
    source: &'static str,
    hook: &HookKind,
    session_index: Option<usize>,
    metrics: &std::collections::HashMap<String, HookMetrics>,
) -> crate::infra::hook::manager::metrics::HookEntry {
    let label = hook_label(hook);
    let uid = hook_unique_id(source, hook, session_index);
    crate::infra::hook::manager::metrics::HookEntry {
        name: hook_name(hook).map(|s| s.to_string()),
        event,
        source,
        hook_type: hook_type_str(hook),
        timeout: hook_timeout(hook),
        on_error: hook_on_error(hook),
        filter: hook_filter(hook).cloned(),
        metrics: metrics.get(&label).cloned(),
        session_index,
        label,
        unique_id: uid,
    }
}