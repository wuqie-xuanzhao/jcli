use super::super::definition::*;
use super::super::executor::{
    execute_hook_with_provider, execute_llm_hook, execute_shell_hook, extract_json_from_llm_output,
    render_prompt_template,
};
use super::super::types::*;
use std::sync::Arc;

#[test]
fn test_execute_shell_hook_echo() {
    let hook = ShellHook {
        name: None,
        command: r#"echo '{"user_input": "hooked"}'"#.to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("original".to_string()),
        ..Default::default()
    };
    let result = execute_shell_hook(&hook, &ctx).expect("should execute shell hook successfully");
    assert_eq!(result.user_input.as_deref(), Some("hooked"));
    assert!(!result.is_halt());
}

#[test]
fn test_execute_shell_hook_empty_output() {
    let hook = ShellHook {
        name: None,
        command: "echo ''".to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext::default();
    let result =
        execute_shell_hook(&hook, &ctx).expect("should execute shell hook with empty output");
    assert!(!result.is_halt());
    assert!(result.user_input.is_none());
}

#[test]
fn test_execute_shell_hook_nonzero_exit() {
    let hook = ShellHook {
        name: None,
        command: "exit 1".to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext::default();
    let result = execute_shell_hook(&hook, &ctx);
    assert!(result.is_err());
}

#[test]
fn test_execute_shell_hook_reads_stdin() {
    let hook = ShellHook {
        name: None,
        command: r#"input=$(cat); event=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('event',''))" 2>/dev/null || echo ""); echo '{"user_input": "got_input"}'"#.to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("test".to_string()),
        ..Default::default()
    };
    let result = execute_shell_hook(&hook, &ctx).expect("should execute shell hook reading stdin");
    assert_eq!(result.user_input.as_deref(), Some("got_input"));
}

#[test]
fn test_execute_builtin_hook() {
    let builtin = BuiltinHook {
        name: "test_hook".to_string(),
        handler: Arc::new(|ctx| {
            ctx.user_input.as_ref().map(|input| HookResult {
                user_input: Some(format!("[hooked] {}", input)),
                ..Default::default()
            })
        }),
    };
    let kind = HookKind::Builtin(builtin);
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("original".to_string()),
        ..Default::default()
    };
    let result =
        execute_hook_with_provider(&kind, &ctx, &None).expect("should execute builtin hook");
    assert_eq!(result.user_input.as_deref(), Some("[hooked] original"));
}

#[test]
fn test_execute_builtin_hook_returns_none() {
    let builtin = BuiltinHook {
        name: "no_op".to_string(),
        handler: Arc::new(|_| None),
    };
    let kind = HookKind::Builtin(builtin);
    let ctx = HookContext::default();
    let result = execute_hook_with_provider(&kind, &ctx, &None)
        .expect("should execute builtin hook returning None");
    assert!(!result.is_halt());
    assert!(result.user_input.is_none());
}

#[test]
fn test_shell_hook_stderr_captured() {
    let hook = ShellHook {
        name: None,
        command: r#"echo '{"user_input": "ok"}'; echo "debug info" >&2"#.to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("test".to_string()),
        ..Default::default()
    };
    let result =
        execute_shell_hook(&hook, &ctx).expect("should execute shell hook with stderr output");
    assert_eq!(result.user_input.as_deref(), Some("ok"));
}

#[test]
fn test_shell_hook_stderr_in_error() {
    let hook = ShellHook {
        name: None,
        command: r#"echo "something went wrong" >&2; exit 1"#.to_string(),
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext::default();
    let result = execute_shell_hook(&hook, &ctx);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("stderr:"), "错误信息应包含 stderr: {}", err);
    assert!(
        err.contains("something went wrong"),
        "错误信息应包含 stderr 内容: {}",
        err
    );
}

#[test]
fn test_render_prompt_template() {
    let template = "事件: {{event}}, 输入: {{user_input}}, 工具: {{tool_name}}";
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("hello".to_string()),
        tool_name: Some("Shell".to_string()),
        ..Default::default()
    };
    let rendered = render_prompt_template(template, &ctx);
    assert!(rendered.contains("pre_send_message"));
    assert!(rendered.contains("hello"));
    assert!(rendered.contains("Shell"));
}

#[test]
fn test_render_prompt_template_empty_fields() {
    let template = "输入: {{user_input}}, 输出: {{assistant_output}}";
    let ctx = HookContext::default();
    let rendered = render_prompt_template(template, &ctx);
    assert_eq!(rendered, "输入: , 输出: ");
}

#[test]
fn test_extract_json_from_llm_output() {
    assert_eq!(
        extract_json_from_llm_output(r#"{"user_input": "test"}"#),
        Some(r#"{"user_input": "test"}"#)
    );
    assert_eq!(
        extract_json_from_llm_output("```json\n{\"user_input\": \"test\"}\n```"),
        Some(r#"{"user_input": "test"}"#)
    );
    assert_eq!(
        extract_json_from_llm_output("Here is the result: {\"action\": \"stop\"}"),
        Some(r#"{"action": "stop"}"#)
    );
    assert_eq!(extract_json_from_llm_output("no json here"), None);
}

#[test]
fn test_llm_hook_no_provider_returns_err() {
    let hook = LlmHook {
        name: None,
        prompt: "test".to_string(),
        model: None,
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
        dir_path: None,
    };
    let ctx = HookContext::default();
    let result = execute_llm_hook(&hook, &ctx, &None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("未注入 provider"));
}
