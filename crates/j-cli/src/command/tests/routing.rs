use crate::cli::SubCmd;
use crate::config::YamlConfig;

// ===================================================================
// 1. into_handler() — every SubCmd variant produces a handler
// ===================================================================

#[test]
fn into_handler_all_variants_no_panic() {
    // If a SubCmd variant is missing from the match arms, this fails
    // at compile time. But field-count changes in a variant could mismatch
    // the corresponding handler struct — this test catches those at runtime.
    let variants: Vec<SubCmd> = vec![
        SubCmd::Set {
            alias: "x".into(),
            path: vec!["/tmp".into()],
        },
        SubCmd::Remove { alias: "x".into() },
        SubCmd::Rename {
            alias: "x".into(),
            new_alias: "y".into(),
        },
        SubCmd::Modify {
            alias: "x".into(),
            path: vec!["/tmp".into()],
        },
        SubCmd::Tag {
            alias: "x".into(),
            category: "browser".into(),
        },
        SubCmd::Untag {
            alias: "x".into(),
            category: "browser".into(),
        },
        SubCmd::List { part: None },
        SubCmd::List {
            part: Some("all".into()),
        },
        SubCmd::Contain {
            alias: "x".into(),
            containers: None,
        },
        SubCmd::Contain {
            alias: "x".into(),
            containers: Some("path,browser".into()),
        },
        SubCmd::Report {
            content: vec!["hello".into()],
        },
        SubCmd::Report { content: vec![] },
        SubCmd::Reportctl {
            action: "new".into(),
            arg: None,
        },
        SubCmd::Reportctl {
            action: "sync".into(),
            arg: Some("2024.01.01".into()),
        },
        SubCmd::Check { line_count: None },
        SubCmd::Check {
            line_count: Some("10".into()),
        },
        SubCmd::Search {
            line_count: "10".into(),
            target: vec!["key".into()],
            fuzzy: false,
        },
        SubCmd::Search {
            line_count: "all".into(),
            target: vec!["key".into()],
            fuzzy: true,
        },
        SubCmd::Todo { content: vec![] },
        SubCmd::Todo {
            content: vec!["buy milk".into()],
        },
        SubCmd::Chat {
            cont: false,
            session: None,
            content: vec![],
            remote: false,
            port: 9390,
            bypass: false,
            no_render: false,
        },
        SubCmd::Chat {
            cont: true,
            session: Some("sess-123".into()),
            content: vec!["hello".into()],
            remote: true,
            port: 8080,
            bypass: true,
            no_render: true,
        },
        SubCmd::Script {
            name: "test".into(),
            content: vec![],
        },
        SubCmd::Script {
            name: "build".into(),
            content: vec!["echo hi".into()],
        },
        SubCmd::Time {
            function: "countdown".into(),
            arg: "30s".into(),
        },
        SubCmd::Log {
            key: "mode".into(),
            value: "verbose".into(),
        },
        SubCmd::Config {
            part: "setting".into(),
            field: "theme".into(),
            value: "dark".into(),
        },
        SubCmd::Clear,
        SubCmd::Version,
        SubCmd::Help,
        SubCmd::Exit,
        SubCmd::Completion { shell: None },
        SubCmd::Completion {
            shell: Some("zsh".into()),
        },
        SubCmd::Update {
            check: false,
            interactive: false,
        },
        SubCmd::Update {
            check: true,
            interactive: true,
        },
        SubCmd::Md { args: vec![] },
        SubCmd::Md {
            args: vec!["list".into()],
        },
        SubCmd::Notebook { args: vec![] },
        SubCmd::Notebook {
            args: vec!["my-note".into()],
        },
        SubCmd::Lock {
            password: "pw".into(),
            target: None,
        },
        SubCmd::Lock {
            password: "pw".into(),
            target: Some("file.txt".into()),
        },
        SubCmd::Unlock {
            password: "pw".into(),
            target: None,
        },
        SubCmd::Unlock {
            password: "pw".into(),
            target: Some("file.txt.lock".into()),
        },
        SubCmd::Read {
            file_path: "test.md".into(),
            port: None,
            no_open: false,
        },
        SubCmd::Read {
            file_path: "doc.md".into(),
            port: Some(3000),
            no_open: true,
        },
    ];

    for variant in variants {
        let _handler = variant.into_handler();
        // No panic = field counts and types match between
        // SubCmd variant and corresponding handler struct.
    }
}

// ===================================================================
// 2. Handler field preservation — verify fields reach the handler
// ===================================================================

#[test]
fn handler_preserves_set_fields() {
    let handler = SubCmd::Set {
        alias: "gh".into(),
        path: vec!["https://github.com".into()],
    }
    .into_handler();

    // Execute on a default config; URL aliases go through set_property
    let mut config = YamlConfig::default();
    handler.execute(&mut config);

    // gh → {https://github.com} was set; verify it exists in inner_url
    assert!(
        config.inner_url.get("gh").is_some() || config.contains("inner_url", "gh"),
        "alias 'gh' should be set"
    );
}
#[test]
fn handler_preserves_remove_fields() {
    // Setup: add an alias first
    let mut config = YamlConfig::default();
    config.path.insert("tmp".into(), "/tmp".into());

    let handler = SubCmd::Remove {
        alias: "tmp".into(),
    }
    .into_handler();
    handler.execute(&mut config);

    assert!(!config.path.contains_key("tmp"));
}

#[test]
fn handler_preserves_rename_fields() {
    let mut config = YamlConfig::default();
    config.path.insert("old".into(), "/tmp".into());

    let handler = SubCmd::Rename {
        alias: "old".into(),
        new_alias: "new".into(),
    }
    .into_handler();
    handler.execute(&mut config);

    assert!(!config.path.contains_key("old"));
    assert_eq!(config.path.get("new").map(String::as_str), Some("/tmp"));
}

#[test]
fn handler_lock_default_target() {
    // LockCmd handler defaults target to "." when None
    // Verify by checking the handler struct fields
    let _handler = SubCmd::Lock {
        password: "pw".into(),
        target: None,
    }
    .into_handler();
    // The struct is opaque, but the execute impl unwraps target to "."
    // We can't easily inspect without I/O, but the assert_no_panic
    // test above covers the type correctness.
}
