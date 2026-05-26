use super::super::definition::*;
use super::super::types::*;

#[test]
fn test_hook_event_roundtrip() {
    for event in HookEvent::all() {
        let s = event.as_str();
        let parsed = HookEvent::parse(s).expect("should parse valid hook event string");
        assert_eq!(*event, parsed);
    }
}

#[test]
fn test_hook_event_from_str_invalid() {
    assert!(HookEvent::parse("unknown_event").is_none());
}

#[test]
fn test_hook_def_default_timeout() {
    let yaml = r#"command: "echo hello""#;
    let def: HookDef = serde_yaml::from_str(yaml).expect("should parse HookDef from YAML string");
    assert_eq!(def.timeout, 10);
    assert_eq!(def.r#type, HookType::Bash);
}

#[test]
fn test_hook_def_to_hook_kind_bash() {
    let def = HookDef {
        r#type: HookType::Bash,
        command: Some("echo test".to_string()),
        prompt: None,
        model: None,
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
    };
    let kind = HookKind::from(def);
    match kind {
        HookKind::Shell(shell) => {
            assert_eq!(shell.command, "echo test");
            assert_eq!(shell.timeout, 5);
        }
        _ => panic!("应该转换为 Shell 变体"),
    }
}

#[test]
fn test_hook_def_to_hook_kind_llm() {
    let def = HookDef {
        r#type: HookType::Llm,
        command: None,
        prompt: Some("检查敏感信息: {{user_input}}".to_string()),
        model: Some("gpt-4o".to_string()),
        timeout: 10,
        retry: 2,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
    };
    let kind = def
        .into_hook_kind()
        .expect("should convert HookDef to HookKind::Llm");
    match kind {
        HookKind::Llm(llm) => {
            assert_eq!(llm.prompt, "检查敏感信息: {{user_input}}");
            assert_eq!(llm.model.as_deref(), Some("gpt-4o"));
            assert_eq!(llm.timeout, 30);
            assert_eq!(llm.retry, 2);
        }
        _ => panic!("应该转换为 Llm 变体"),
    }
}

#[test]
fn test_hook_def_llm_explicit_timeout() {
    let def = HookDef {
        r#type: HookType::Llm,
        command: None,
        prompt: Some("test prompt".to_string()),
        model: None,
        timeout: 60,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
    };
    let kind = def
        .into_hook_kind()
        .expect("should convert HookDef with explicit timeout");
    match kind {
        HookKind::Llm(llm) => {
            assert_eq!(llm.timeout, 60);
        }
        _ => panic!("应该转换为 Llm 变体"),
    }
}

#[test]
fn test_hook_def_yaml_with_type() {
    let yaml = r#"
type: llm
prompt: "检查敏感信息"
model: gpt-4o
timeout: 30
retry: 2
"#;
    let def: HookDef =
        serde_yaml::from_str(yaml).expect("should parse HookDef from YAML with type field");
    assert_eq!(def.r#type, HookType::Llm);
    assert_eq!(def.prompt.as_deref(), Some("检查敏感信息"));
    assert_eq!(def.model.as_deref(), Some("gpt-4o"));
    assert_eq!(def.timeout, 30);
    assert_eq!(def.retry, 2);
}

#[test]
fn test_hook_result_empty_json() {
    let result: HookResult =
        serde_json::from_str("{}").expect("should parse empty JSON as HookResult");
    assert!(!result.is_halt());
    assert!(result.messages.is_none());
    assert!(result.user_input.is_none());
}

#[test]
fn test_hook_result_with_stop() {
    let json = r#"{"action": "stop"}"#;
    let result: HookResult =
        serde_json::from_str(json).expect("should parse stop action JSON result");
    assert!(result.is_stop());
}

#[test]
fn test_hook_result_with_action_stop() {
    let json = r#"{"action": "stop"}"#;
    let result: HookResult =
        serde_json::from_str(json).expect("should parse action=stop HookResult");
    assert!(result.is_stop());
    assert!(!result.is_skip());
}

#[test]
fn test_hook_result_with_action_skip() {
    let json = r#"{"action": "skip"}"#;
    let result: HookResult =
        serde_json::from_str(json).expect("should parse action=skip HookResult");
    assert!(result.is_skip());
    assert!(!result.is_stop());
}

#[test]
fn test_hook_result_with_user_input() {
    let json = r#"{"user_input": "[modified] hello"}"#;
    let result: HookResult =
        serde_json::from_str(json).expect("should parse HookResult with user_input");
    assert_eq!(result.user_input.as_deref(), Some("[modified] hello"));
}

#[test]
fn test_hook_context_serialization() {
    let ctx = HookContext {
        event: HookEvent::PreSendMessage,
        user_input: Some("hello".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_string(&ctx).expect("should serialize HookContext to JSON string");
    assert!(json.contains("pre_send_message"));
    assert!(json.contains("hello"));
    assert!(json.contains("user_input"));
    assert!(!json.contains("messages"));
    assert!(!json.contains("tool_name"));
}

#[test]
fn test_new_hook_events_roundtrip() {
    for event in [
        HookEvent::Stop,
        HookEvent::PreMicroCompact,
        HookEvent::PostMicroCompact,
        HookEvent::PreAutoCompact,
        HookEvent::PostAutoCompact,
        HookEvent::PostToolExecutionFailure,
    ] {
        let s = event.as_str();
        let parsed = HookEvent::parse(s).expect("should parse new hook event strings");
        assert_eq!(event, parsed);
    }
}

#[test]
fn test_hook_result_retry_feedback() {
    let json = r#"{"action": "stop", "retry_feedback": "请修正敏感信息"}"#;
    let result: HookResult =
        serde_json::from_str(json).expect("should parse HookResult with retry_feedback");
    assert!(result.is_stop());
    assert_eq!(result.retry_feedback.as_deref(), Some("请修正敏感信息"));
}

#[test]
fn test_hook_result_action_stop_with_retry_feedback() {
    let json = r#"{"action": "stop", "retry_feedback": "请修正敏感信息"}"#;
    let result: HookResult =
        serde_json::from_str(json).expect("should parse action=stop with retry_feedback");
    assert!(result.is_stop());
    assert_eq!(result.retry_feedback.as_deref(), Some("请修正敏感信息"));
}

#[test]
fn test_hook_result_additional_context() {
    let json = r#"{"additional_context": "必须保留宪法规则"}"#;
    let result: HookResult =
        serde_json::from_str(json).expect("should parse HookResult with additional_context");
    assert_eq!(
        result.additional_context.as_deref(),
        Some("必须保留宪法规则")
    );
}

#[test]
fn test_hook_result_system_message() {
    let json = r#"{"system_message": "纠查官已审查"}"#;
    let result: HookResult =
        serde_json::from_str(json).expect("should parse HookResult with system_message");
    assert_eq!(result.system_message.as_deref(), Some("纠查官已审查"));
}

#[test]
fn test_hook_result_tool_error() {
    let json = r#"{"tool_error": "权限不足"}"#;
    let result: HookResult =
        serde_json::from_str(json).expect("should parse HookResult with tool_error");
    assert_eq!(result.tool_error.as_deref(), Some("权限不足"));
}

#[test]
fn test_switch_model_field_removed() {
    let json = r#"{"user_input": "test", "_switch_model": "gpt-4"}"#;
    let result: HookResult =
        serde_json::from_str(json).expect("should parse result with _switch_model field");
    assert_eq!(result.user_input.as_deref(), Some("test"));
}

#[test]
fn test_hook_context_new_fields() {
    let ctx = HookContext {
        event: HookEvent::PreAutoCompact,
        tool_error: None,
        ..Default::default()
    };
    let json = serde_json::to_string(&ctx).expect("should serialize HookContext with new fields");
    assert!(json.contains("pre_auto_compact"));
    assert!(!json.contains("tool_error"));
}

#[test]
fn test_hook_type_display() {
    assert_eq!(format!("{}", HookType::Bash), "bash");
    assert_eq!(format!("{}", HookType::Llm), "llm");
}

#[test]
fn test_hook_type_yaml_parsing() {
    let yaml_bash = r#"command: "echo hello""#;
    let def: HookDef =
        serde_yaml::from_str(yaml_bash).expect("should parse HookDef as bash type from YAML");
    assert_eq!(def.r#type, HookType::Bash);

    let yaml_llm = r#"
type: llm
prompt: "check this""#;
    let def: HookDef =
        serde_yaml::from_str(yaml_llm).expect("should parse HookDef as llm type from YAML");
    assert_eq!(def.r#type, HookType::Llm);
    assert_eq!(def.prompt.as_deref(), Some("check this"));
}

#[test]
fn test_hook_def_bash_missing_command() {
    let def = HookDef {
        r#type: HookType::Bash,
        command: None,
        prompt: None,
        model: None,
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
    };
    assert!(def.into_hook_kind().is_err());
}

#[test]
fn test_hook_def_llm_missing_prompt() {
    let def = HookDef {
        r#type: HookType::Llm,
        command: None,
        prompt: None,
        model: None,
        timeout: 5,
        retry: 0,
        on_error: OnError::Skip,
        filter: HookFilter::default(),
    };
    assert!(def.into_hook_kind().is_err());
}
