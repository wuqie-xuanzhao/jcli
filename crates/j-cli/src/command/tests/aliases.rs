use super::parse_cli;
use crate::cli::SubCmd;

// ===================================================================
// Command alias tests — verify short/long CLI names work
// ===================================================================

#[test]
fn parse_set_alias_s() {
    let cli = parse_cli(&["s", "gh", "https://github.com"]);
    assert!(matches!(cli.command.unwrap(), SubCmd::Set { .. }));
}

#[test]
fn parse_remove_long_name() {
    let cli = parse_cli(&["remove", "gh"]);
    assert!(matches!(cli.command.unwrap(), SubCmd::Remove { .. }));
}

#[test]
fn parse_modify_alias_mf() {
    let cli = parse_cli(&["mf", "gh", "https://new-url.com"]);
    let SubCmd::Modify { alias, path } = cli.command.unwrap() else {
        panic!("Expected Modify");
    };
    assert_eq!(alias, "gh");
    assert_eq!(path, vec!["https://new-url.com"]);
}

#[test]
fn parse_tag_alias_t() {
    let cli = parse_cli(&["t", "gh", "browser"]);
    let SubCmd::Tag { alias, category } = cli.command.unwrap() else {
        panic!("Expected Tag");
    };
    assert_eq!(alias, "gh");
    assert_eq!(category, "browser");
}

#[test]
fn parse_untag_alias_ut() {
    let cli = parse_cli(&["ut", "gh", "browser"]);
    let SubCmd::Untag { alias, category } = cli.command.unwrap() else {
        panic!("Expected Untag");
    };
    assert_eq!(alias, "gh");
    assert_eq!(category, "browser");
}

#[test]
fn parse_report_alias_r() {
    let cli = parse_cli(&["r", "did some work"]);
    let SubCmd::Report { content } = cli.command.unwrap() else {
        panic!("Expected Report");
    };
    assert_eq!(content, vec!["did some work"]);
}

#[test]
fn parse_reportctl_alias_rctl() {
    let cli = parse_cli(&["rctl", "sync", "2024.01.01"]);
    let SubCmd::Reportctl { action, arg } = cli.command.unwrap() else {
        panic!("Expected Reportctl");
    };
    assert_eq!(action, "sync");
    assert_eq!(arg, Some("2024.01.01".into()));
}

#[test]
fn parse_search_with_aliases() {
    for alias in &["search", "select", "look", "sch"] {
        let cli = parse_cli(&[alias, "10", "keyword"]);
        let SubCmd::Search {
            line_count,
            target,
            fuzzy,
        } = cli.command.unwrap()
        else {
            panic!("Expected Search for alias '{alias}'");
        };
        assert_eq!(line_count, "10");
        assert_eq!(target, vec!["keyword"]);
        assert!(!fuzzy);
    }
}

#[test]
fn parse_script_alias_sc() {
    let cli = parse_cli(&["sc", "build", "cargo build"]);
    let SubCmd::Script { name, content } = cli.command.unwrap() else {
        panic!("Expected Script");
    };
    assert_eq!(name, "build");
    assert_eq!(content, vec!["cargo build"]);
}

#[test]
fn parse_config_alias_cfg() {
    let cli = parse_cli(&["cfg", "setting", "color_mode", "dark"]);
    let SubCmd::Config { part, field, value } = cli.command.unwrap() else {
        panic!("Expected Config");
    };
    assert_eq!(part, "setting");
    assert_eq!(field, "color_mode");
    assert_eq!(value, "dark");
}

#[test]
fn parse_clear_alias_cls() {
    let cli = parse_cli(&["cls"]);
    assert!(matches!(cli.command.unwrap(), SubCmd::Clear));
}

#[test]
fn parse_version_alias_v() {
    let cli = parse_cli(&["v"]);
    assert!(matches!(cli.command.unwrap(), SubCmd::Version));
}

#[test]
fn parse_help_alias_h() {
    let cli = parse_cli(&["h"]);
    assert!(matches!(cli.command.unwrap(), SubCmd::Help));
}

#[test]
fn parse_exit_aliases() {
    for alias in &["exit", "q", "quit"] {
        let cli = parse_cli(&[alias]);
        assert!(
            matches!(cli.command.unwrap(), SubCmd::Exit),
            "alias '{}' failed",
            alias
        );
    }
}

#[test]
fn parse_update_alias_up() {
    let cli = parse_cli(&["up"]);
    let SubCmd::Update {
        check, interactive, ..
    } = cli.command.unwrap()
    else {
        panic!("Expected Update");
    };
    assert!(!check);
    assert!(!interactive);
}

#[test]
fn parse_md_alias_markdown() {
    let cli = parse_cli(&["markdown", "list"]);
    let SubCmd::Md { args } = cli.command.unwrap() else {
        panic!("Expected Md");
    };
    assert_eq!(args, vec!["list"]);
}

#[test]
fn parse_notebook_alias_nb() {
    let cli = parse_cli(&["nb", "my-note"]);
    let SubCmd::Notebook { args } = cli.command.unwrap() else {
        panic!("Expected Notebook");
    };
    assert_eq!(args, vec!["my-note"]);
}

#[test]
fn parse_unlock_alias_uk() {
    let cli = parse_cli(&["uk", "pw"]);
    let SubCmd::Unlock { password, target } = cli.command.unwrap() else {
        panic!("Expected Unlock");
    };
    assert_eq!(password, "pw");
    assert_eq!(target, None);
}

#[test]
fn parse_read_alias_rd() {
    let cli = parse_cli(&["rd", "doc.md"]);
    let SubCmd::Read {
        file_path,
        port,
        no_open,
    } = cli.command.unwrap()
    else {
        panic!("Expected Read");
    };
    assert_eq!(file_path, "doc.md");
    assert_eq!(port, None);
    assert!(!no_open);
}
