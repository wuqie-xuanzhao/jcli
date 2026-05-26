use super::{parse_cli, parse_cli_err};
use crate::cli::SubCmd;

// ===================================================================
// Basic command parsing
// ===================================================================

#[test]
fn parse_set_basic() {
    let cli = parse_cli(&["set", "gh", "https://github.com"]);
    let SubCmd::Set { alias, path } = cli.command.unwrap() else {
        panic!("Expected Set");
    };
    assert_eq!(alias, "gh");
    assert_eq!(path, vec!["https://github.com"]);
}

#[test]
fn parse_set_multi_word_path() {
    let cli = parse_cli(&["set", "note", "C:\\My", "Documents\\notes"]);
    let SubCmd::Set { alias, path } = cli.command.unwrap() else {
        panic!("Expected Set");
    };
    assert_eq!(alias, "note");
    assert_eq!(path, vec!["C:\\My", "Documents\\notes"]);
}

#[test]
fn parse_remove_basic() {
    let cli = parse_cli(&["rm", "gh"]);
    let SubCmd::Remove { alias } = cli.command.unwrap() else {
        panic!("Expected Remove");
    };
    assert_eq!(alias, "gh");
}

#[test]
fn parse_rename_basic() {
    let cli = parse_cli(&["rn", "old", "new"]);
    let SubCmd::Rename { alias, new_alias } = cli.command.unwrap() else {
        panic!("Expected Rename");
    };
    assert_eq!(alias, "old");
    assert_eq!(new_alias, "new");
}

#[test]
fn parse_list_no_args() {
    let cli = parse_cli(&["ls"]);
    let SubCmd::List { part } = cli.command.unwrap() else {
        panic!("Expected List");
    };
    assert_eq!(part, None);
}

#[test]
fn parse_list_with_part() {
    let cli = parse_cli(&["list", "all"]);
    let SubCmd::List { part } = cli.command.unwrap() else {
        panic!("Expected List");
    };
    assert_eq!(part, Some("all".into()));
}

#[test]
fn parse_contain_basic() {
    let cli = parse_cli(&["find", "gh"]);
    let SubCmd::Contain {
        alias, containers, ..
    } = cli.command.unwrap()
    else {
        panic!("Expected Contain");
    };
    assert_eq!(alias, "gh");
    assert_eq!(containers, None);
}

#[test]
fn parse_contain_with_containers() {
    let cli = parse_cli(&["contain", "gh", "path,browser"]);
    let SubCmd::Contain {
        alias, containers, ..
    } = cli.command.unwrap()
    else {
        panic!("Expected Contain");
    };
    assert_eq!(alias, "gh");
    assert_eq!(containers, Some("path,browser".into()));
}

#[test]
fn parse_reportctl_new() {
    let cli = parse_cli(&["reportctl", "new"]);
    let SubCmd::Reportctl { action, arg } = cli.command.unwrap() else {
        panic!("Expected Reportctl");
    };
    assert_eq!(action, "new");
    assert_eq!(arg, None);
}

#[test]
fn parse_check_default() {
    let cli = parse_cli(&["c"]);
    let SubCmd::Check { line_count } = cli.command.unwrap() else {
        panic!("Expected Check");
    };
    assert_eq!(line_count, None);
}

#[test]
fn parse_check_with_count() {
    let cli = parse_cli(&["check", "20"]);
    let SubCmd::Check { line_count } = cli.command.unwrap() else {
        panic!("Expected Check");
    };
    assert_eq!(line_count, Some("20".into()));
}

#[test]
fn parse_search_fuzzy_flag_consumed_as_target() {
    // With allow_hyphen_values=true on target, -f is consumed as a target value,
    // not as the fuzzy flag. This is the current clap behavior.
    let cli = parse_cli(&["search", "all", "keyword", "-f"]);
    let SubCmd::Search {
        line_count,
        target,
        fuzzy,
    } = cli.command.unwrap()
    else {
        panic!("Expected Search");
    };
    assert_eq!(line_count, "all");
    assert_eq!(target, vec!["keyword", "-f"]);
    assert!(!fuzzy);
}

#[test]
fn parse_search_long_flag_consumed_as_target() {
    let cli = parse_cli(&["search", "5", "key", "--fuzzy"]);
    let SubCmd::Search { target, fuzzy, .. } = cli.command.unwrap() else {
        panic!("Expected Search");
    };
    assert_eq!(target, vec!["key", "--fuzzy"]);
    assert!(!fuzzy);
}

#[test]
fn parse_todo_empty() {
    let cli = parse_cli(&["td"]);
    let SubCmd::Todo { content } = cli.command.unwrap() else {
        panic!("Expected Todo");
    };
    assert!(content.is_empty());
}

#[test]
fn parse_todo_with_content() {
    let cli = parse_cli(&["todo", "buy", "milk"]);
    let SubCmd::Todo { content } = cli.command.unwrap() else {
        panic!("Expected Todo");
    };
    assert_eq!(content, vec!["buy", "milk"]);
}

#[test]
fn parse_chat_defaults() {
    let cli = parse_cli(&["ai", "hello"]);
    let SubCmd::Chat {
        cont,
        session,
        content,
        remote,
        port,
        bypass,
        no_render,
    } = cli.command.unwrap()
    else {
        panic!("Expected Chat");
    };
    assert!(!cont);
    assert_eq!(session, None);
    assert_eq!(content, vec!["hello"]);
    assert!(!remote);
    assert_eq!(port, 9390);
    assert!(!bypass);
    assert!(!no_render);
}

#[test]
fn parse_chat_continue_flag() {
    let cli = parse_cli(&["chat", "-c", "continue this"]);
    let SubCmd::Chat { cont, .. } = cli.command.unwrap() else {
        panic!("Expected Chat");
    };
    assert!(cont);
}

#[test]
fn parse_chat_session_flag() {
    let cli = parse_cli(&["chat", "--session", "abc123", "hello"]);
    let SubCmd::Chat { session, .. } = cli.command.unwrap() else {
        panic!("Expected Chat");
    };
    assert_eq!(session, Some("abc123".into()));
}

#[test]
fn parse_chat_remote_flag() {
    let cli = parse_cli(&["chat", "--remote", "hello"]);
    let SubCmd::Chat { remote, .. } = cli.command.unwrap() else {
        panic!("Expected Chat");
    };
    assert!(remote);
}

#[test]
fn parse_chat_custom_port() {
    let cli = parse_cli(&["chat", "--port", "8080", "hello"]);
    let SubCmd::Chat { port, .. } = cli.command.unwrap() else {
        panic!("Expected Chat");
    };
    assert_eq!(port, 8080);
}

#[test]
fn parse_chat_bypass_and_no_render() {
    let cli = parse_cli(&["chat", "--bypass", "--no-render", "cmd"]);
    let SubCmd::Chat {
        bypass, no_render, ..
    } = cli.command.unwrap()
    else {
        panic!("Expected Chat");
    };
    assert!(bypass);
    assert!(no_render);
}

#[test]
fn parse_time_basic() {
    let cli = parse_cli(&["time", "countdown", "5m"]);
    let SubCmd::Time { function, arg } = cli.command.unwrap() else {
        panic!("Expected Time");
    };
    assert_eq!(function, "countdown");
    assert_eq!(arg, "5m");
}

#[test]
fn parse_log_basic() {
    let cli = parse_cli(&["log", "mode", "verbose"]);
    let SubCmd::Log { key, value } = cli.command.unwrap() else {
        panic!("Expected Log");
    };
    assert_eq!(key, "mode");
    assert_eq!(value, "verbose");
}

#[test]
fn parse_completion_no_shell() {
    let cli = parse_cli(&["completion"]);
    let SubCmd::Completion { shell } = cli.command.unwrap() else {
        panic!("Expected Completion");
    };
    assert_eq!(shell, None);
}

#[test]
fn parse_completion_with_shell() {
    let cli = parse_cli(&["completion", "zsh"]);
    let SubCmd::Completion { shell } = cli.command.unwrap() else {
        panic!("Expected Completion");
    };
    assert_eq!(shell, Some("zsh".into()));
}

#[test]
fn parse_update_check_only() {
    let cli = parse_cli(&["update", "-c"]);
    let SubCmd::Update { check, .. } = cli.command.unwrap() else {
        panic!("Expected Update");
    };
    assert!(check);
}

#[test]
fn parse_lock_basic() {
    let cli = parse_cli(&["lk", "mypassword"]);
    let SubCmd::Lock { password, target } = cli.command.unwrap() else {
        panic!("Expected Lock");
    };
    assert_eq!(password, "mypassword");
    assert_eq!(target, None);
}

#[test]
fn parse_lock_with_target() {
    let cli = parse_cli(&["lock", "pw", "secret.txt"]);
    let SubCmd::Lock { password, target } = cli.command.unwrap() else {
        panic!("Expected Lock");
    };
    assert_eq!(password, "pw");
    assert_eq!(target, Some("secret.txt".into()));
}

#[test]
fn parse_unlock_with_target() {
    let cli = parse_cli(&["unlock", "pw", "secret.txt.lock"]);
    let SubCmd::Unlock { password, target } = cli.command.unwrap() else {
        panic!("Expected Unlock");
    };
    assert_eq!(password, "pw");
    assert_eq!(target, Some("secret.txt.lock".into()));
}

#[test]
fn parse_read_with_options() {
    let cli = parse_cli(&["read", "doc.md", "--port", "3000", "--no-open"]);
    let SubCmd::Read {
        file_path,
        port,
        no_open,
    } = cli.command.unwrap()
    else {
        panic!("Expected Read");
    };
    assert_eq!(file_path, "doc.md");
    assert_eq!(port, Some(3000));
    assert!(no_open);
}

// ===================================================================
// Edge cases: missing required args
// ===================================================================

#[test]
fn set_missing_path_uses_trailing() {
    // trailing_var_arg allows zero path args; clap won't error
    let cli = parse_cli(&["set", "gh"]);
    let SubCmd::Set { alias, path } = cli.command.unwrap() else {
        panic!("Expected Set");
    };
    assert_eq!(alias, "gh");
    assert!(path.is_empty());
}

#[test]
fn remove_missing_alias_is_error() {
    let err = parse_cli_err(&["rm"]);
    let msg = err.to_string();
    assert!(
        msg.contains("alias") || msg.contains("required"),
        "Expected error about missing alias, got: {msg}"
    );
}

#[test]
fn time_missing_args_is_error() {
    let err = parse_cli_err(&["time"]);
    let msg = err.to_string();
    assert!(msg.contains("required") || msg.contains("function"));
}

#[test]
fn lock_missing_password_is_error() {
    let err = parse_cli_err(&["lock"]);
    let msg = err.to_string();
    assert!(msg.contains("password") || msg.contains("required"));
}

#[test]
fn read_missing_file_path_is_error() {
    let err = parse_cli_err(&["read"]);
    let msg = err.to_string();
    assert!(msg.contains("file_path") || msg.contains("required"));
}

// ===================================================================
// No subcommand — args fallback
// ===================================================================

#[test]
fn no_subcommand_collects_trailing_args() {
    let cli = parse_cli(&["unknown-alias", "arg1", "arg2"]);
    assert!(cli.command.is_none());
    assert_eq!(cli.args, vec!["unknown-alias", "arg1", "arg2"]);
}
