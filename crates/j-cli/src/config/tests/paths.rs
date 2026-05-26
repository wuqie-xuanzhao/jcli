use super::*;
use std::path::PathBuf;

// ========================================================================
// report_file_path
// ========================================================================

#[test]
fn test_report_file_path_custom_path_is_used() {
    let mut config = YamlConfig::default();
    config
        .report
        .insert(config_key::WEEK_REPORT.into(), "/custom/report.md".into());
    let path = config.report_file_path();
    assert_eq!(path, PathBuf::from("/custom/report.md"));
}

#[test]
fn test_report_file_path_default_when_not_set() {
    let config = YamlConfig::default();
    let path = config.report_file_path();
    let expected = YamlConfig::report_dir().join(REPORT_DEFAULT_FILE);
    assert_eq!(path, expected);
}

#[test]
fn test_report_file_path_empty_custom_uses_default() {
    let mut config = YamlConfig::default();
    config
        .report
        .insert(config_key::WEEK_REPORT.into(), String::new());
    let path = config.report_file_path();
    let expected = YamlConfig::report_dir().join(REPORT_DEFAULT_FILE);
    assert_eq!(path, expected);
}

#[test]
fn test_report_file_path_expands_tilde() {
    let home = dirs::home_dir().expect("home dir should exist");
    let mut config = YamlConfig::default();
    config.report.insert(
        config_key::WEEK_REPORT.into(),
        "~/reports/my_report.md".into(),
    );
    let path = config.report_file_path();
    assert_eq!(path, home.join("reports/my_report.md"));
}

#[test]
fn test_report_file_path_expands_tilde_alone() {
    let home = dirs::home_dir().expect("home dir should exist");
    let mut config = YamlConfig::default();
    config
        .report
        .insert(config_key::WEEK_REPORT.into(), "~".into());
    let path = config.report_file_path();
    assert_eq!(path, home);
}
