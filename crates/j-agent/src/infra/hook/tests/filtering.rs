use super::super::definition::*;
use super::super::manager::*;
use super::super::types::*;

#[test]
fn test_hook_filter_tool_matcher() {
    let filter = HookFilter {
        tool_name: None,
        tool_matcher: Some("Shell".to_string()),
        model_prefix: None,
    };
    assert!(!filter.is_empty());

    let ctx = HookContext {
        event: HookEvent::PreToolExecution,
        tool_name: Some("Shell".to_string()),
        ..Default::default()
    };
    assert!(filter.matches(&ctx));

    let ctx = HookContext {
        event: HookEvent::PreToolExecution,
        tool_name: Some("Write".to_string()),
        ..Default::default()
    };
    assert!(!filter.matches(&ctx));

    let ctx = HookContext {
        event: HookEvent::PreToolExecution,
        ..Default::default()
    };
    assert!(!filter.matches(&ctx));
}

#[test]
fn test_hook_filter_tool_name_priority_over_matcher() {
    let filter = HookFilter {
        tool_name: Some("Shell".to_string()),
        tool_matcher: Some("Write|Edit".to_string()),
        model_prefix: None,
    };
    let ctx = HookContext {
        event: HookEvent::PreToolExecution,
        tool_name: Some("Write".to_string()),
        ..Default::default()
    };
    assert!(!filter.matches(&ctx));
}

#[test]
fn test_hook_filter_tool_matcher_yaml() {
    let yaml = r#"tool_matcher: "Shell""#;
    let filter: HookFilter = serde_yaml::from_str(yaml).expect("should parse HookFilter from YAML");
    assert_eq!(filter.tool_matcher.as_deref(), Some("Shell"));
    assert!(filter.tool_name.is_none());
}

#[test]
fn test_on_error_default_is_skip() {
    let yaml = r#"command: "exit 1"
timeout: 5"#;
    let def: HookDef =
        serde_yaml::from_str(yaml).expect("should parse HookDef with default on_error");
    assert_eq!(def.on_error, OnError::Skip);
}

#[test]
fn test_on_error_yaml_parsing() {
    let yaml_skip = r#"command: "echo test"
on_error: skip"#;
    let def: HookDef =
        serde_yaml::from_str(yaml_skip).expect("should parse on_error:skip from YAML");
    assert_eq!(def.on_error, OnError::Skip);

    let yaml_stop = r#"command: "echo test"
on_error: stop"#;
    let def: HookDef =
        serde_yaml::from_str(yaml_stop).expect("should parse on_error:stop from YAML");
    assert_eq!(def.on_error, OnError::Stop);
}

#[test]
fn test_hook_entry_hook_type() {
    let mut manager = HookManager::default();

    manager.register_builtin(HookEvent::PreSendMessage, "test", |_| None);

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("echo test".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Llm,
            command: None,
            prompt: Some("check content".to_string()),
            model: None,
            timeout: 30,
            retry: 1,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let hooks = manager.list_hooks();
    assert_eq!(hooks.len(), 3);
    assert_eq!(hooks[0].hook_type, "builtin");
    assert_eq!(hooks[1].hook_type, "bash");
    assert_eq!(hooks[2].hook_type, "llm");
}

#[test]
fn test_hook_entry_session_index() {
    let mut manager = HookManager::default();

    manager.register_builtin(HookEvent::PreSendMessage, "test", |_| None);

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("echo first".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("echo second".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Stop,
            filter: HookFilter::default(),
        },
    );

    let hooks = manager.list_hooks();
    assert_eq!(hooks.len(), 3);

    assert_eq!(hooks[0].source, "builtin");
    assert!(hooks[0].session_index.is_none());
    assert!(hooks[0].on_error.is_none());

    assert_eq!(hooks[1].source, "session");
    assert_eq!(hooks[1].session_index, Some(0));
    assert_eq!(hooks[1].on_error, Some(OnError::Skip));

    assert_eq!(hooks[2].source, "session");
    assert_eq!(hooks[2].session_index, Some(1));
    assert_eq!(hooks[2].on_error, Some(OnError::Stop));
}
