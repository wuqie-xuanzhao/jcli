use crate::infra::hook::definition::*;
use crate::infra::hook::manager::*;
use crate::infra::hook::types::*;

#[test]
fn test_hook_manager_empty() {
    let manager = HookManager::default();
    assert!(manager.list_hooks().is_empty());
    let result = manager.execute(HookEvent::PreSendMessage, HookContext::default(), &[]);
    assert!(result.is_none());
}

#[test]
fn test_hook_manager_session_hooks() {
    let mut manager = HookManager::default();
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "session_hooked"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let hooks = manager.list_hooks();
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].source, "session");

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                user_input: Some("original".to_string()),
                ..Default::default()
            },
            &[],
        )
        .expect("should execute session hooks via manager");
    assert_eq!(result.user_input.as_deref(), Some("session_hooked"));
}

#[test]
fn test_hook_manager_builtin_hooks() {
    let mut manager = HookManager::default();
    manager.register_builtin(HookEvent::PreSendMessage, "test_builtin", |ctx| {
        ctx.user_input.as_ref().map(|input| HookResult {
            user_input: Some(format!("[builtin] {}", input)),
            ..Default::default()
        })
    });

    let hooks = manager.list_hooks();
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].source, "builtin");
    assert!(hooks[0].label.contains("test_builtin"));

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                user_input: Some("hello".to_string()),
                ..Default::default()
            },
            &[],
        )
        .expect("should execute builtin hook via manager");
    assert_eq!(result.user_input.as_deref(), Some("[builtin] hello"));
}

#[test]
fn test_hook_manager_builtin_before_session() {
    let mut manager = HookManager::default();
    manager.register_builtin(HookEvent::PreSendMessage, "prefix", |ctx| {
        ctx.user_input.as_ref().map(|input| HookResult {
            user_input: Some(format!("[builtin] {}", input)),
            ..Default::default()
        })
    });
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "session_overridden"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                user_input: Some("original".to_string()),
                ..Default::default()
            },
            &[],
        )
        .expect("should execute builtin before session hook");
    assert_eq!(result.user_input.as_deref(), Some("session_overridden"));
}

#[test]
fn test_hook_manager_remove_session_hook() {
    let mut manager = HookManager::default();
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
    assert_eq!(manager.list_hooks().len(), 1);

    assert!(manager.remove_session_hook(HookEvent::PreSendMessage, 0));
    assert!(manager.list_hooks().is_empty());

    assert!(!manager.remove_session_hook(HookEvent::PreSendMessage, 0));
}

#[test]
fn test_hook_chain_execution() {
    let mut manager = HookManager::default();

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "first"}'"#.to_string()),
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
            command: Some(r#"echo '{"user_input": "second"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                user_input: Some("original".to_string()),
                ..Default::default()
            },
            &[],
        )
        .expect("should execute hook chain via manager");

    assert_eq!(result.user_input.as_deref(), Some("second"));
}

#[test]
fn test_hook_stop_stops_chain() {
    let mut manager = HookManager::default();

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("exit 1".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Stop,
            filter: HookFilter::default(),
        },
    );
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "should_not_reach"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                ..Default::default()
            },
            &[],
        )
        .expect("should execute stop chain via manager");

    assert!(result.is_halt());
    assert!(result.user_input.is_none());
}

#[test]
fn test_builtin_hook_clone() {
    let mut manager = HookManager::default();
    manager.register_builtin(HookEvent::PreLlmRequest, "test_clone", |_| {
        Some(HookResult::default())
    });
    let cloned = manager.clone();
    assert_eq!(cloned.list_hooks().len(), 1);
}

#[test]
fn test_on_error_skip_continues_chain() {
    let mut manager = HookManager::default();

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("exit 1".to_string()),
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
            command: Some(r#"echo '{"user_input": "survived"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                user_input: Some("original".to_string()),
                ..Default::default()
            },
            &[],
        )
        .expect("should execute skip-and-continue chain");

    assert!(!result.is_halt());
    assert_eq!(result.user_input.as_deref(), Some("survived"));
}

#[test]
fn test_on_error_stop_stops_chain() {
    let mut manager = HookManager::default();

    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some("exit 1".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Stop,
            filter: HookFilter::default(),
        },
    );
    manager.register_session_hook(
        HookEvent::PreSendMessage,
        HookDef {
            r#type: HookType::Bash,
            command: Some(r#"echo '{"user_input": "should_not_reach"}'"#.to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        },
    );

    let result = manager
        .execute(
            HookEvent::PreSendMessage,
            HookContext {
                event: HookEvent::PreSendMessage,
                ..Default::default()
            },
            &[],
        )
        .expect("should execute stop-stops-chain via manager");

    assert!(result.is_halt());
    assert!(result.user_input.is_none());
}
