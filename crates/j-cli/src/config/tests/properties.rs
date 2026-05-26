use super::*;
use std::collections::BTreeMap;

// ========================================================================
// is_verbose
// ========================================================================

#[test]
fn test_is_verbose_when_mode_verbose_returns_true() {
    let mut config = YamlConfig::default();
    config
        .log
        .insert(config_key::MODE.into(), config_key::VERBOSE.into());
    assert!(config.is_verbose(), "verbose mode should return true");
}

#[test]
fn test_is_verbose_when_mode_concise_returns_false() {
    let mut config = YamlConfig::default();
    config
        .log
        .insert(config_key::MODE.into(), config_key::CONCISE.into());
    assert!(!config.is_verbose(), "concise mode should return false");
}

#[test]
fn test_is_verbose_when_no_mode_key_returns_false() {
    let config = YamlConfig::default();
    assert!(!config.is_verbose(), "missing mode key should return false");
}

// ========================================================================
// get_section / get_section_mut
// ========================================================================

#[test]
fn test_get_section_all_valid_sections() {
    let config = make_populated_config();
    assert!(config.get_section(section::PATH).is_some());
    assert!(config.get_section(section::INNER_URL).is_some());
    assert!(config.get_section(section::OUTER_URL).is_some());
    assert!(config.get_section(section::EDITOR).is_some());
    assert!(config.get_section(section::BROWSER).is_some());
    assert!(config.get_section(section::VPN).is_some());
    assert!(config.get_section(section::SCRIPT).is_some());
    assert!(config.get_section(section::VERSION).is_some());
    assert!(config.get_section(section::SETTING).is_some());
    assert!(config.get_section(section::LOG).is_some());
    assert!(config.get_section(section::REPORT).is_some());
}

#[test]
fn test_get_section_invalid_returns_none() {
    let config = make_populated_config();
    assert!(config.get_section("nonexistent").is_none());
    assert!(config.get_section("").is_none());
    assert!(config.get_section("random_string").is_none());
}

#[test]
fn test_get_section_mut_allows_modification() {
    let mut config = make_populated_config();
    let path_map = config
        .get_section_mut(section::PATH)
        .expect("path section exists");
    path_map.insert("new_key".into(), "new_value".into());
    assert_eq!(
        config.get_property(section::PATH, "new_key"),
        Some(&"new_value".to_string())
    );
}

#[test]
fn test_get_section_mut_invalid_returns_none() {
    let mut config = make_populated_config();
    assert!(config.get_section_mut("nonexistent").is_none());
}

// ========================================================================
// contains / get_property
// ========================================================================

#[test]
fn test_contains_existing_key_returns_true() {
    let config = make_populated_config();
    assert!(config.contains(section::PATH, "home"));
    assert!(config.contains(section::BROWSER, "chrome"));
    assert!(config.contains(section::EDITOR, "code"));
}

#[test]
fn test_contains_missing_key_returns_false() {
    let config = make_populated_config();
    assert!(!config.contains(section::PATH, "nonexistent"));
    assert!(!config.contains(section::BROWSER, "firefox"));
}

#[test]
fn test_contains_invalid_section_returns_false() {
    let config = make_populated_config();
    assert!(!config.contains("nonexistent", "home"));
    assert!(!config.contains("", "home"));
}

#[test]
fn test_get_property_existing_key_returns_value() {
    let config = make_populated_config();
    assert_eq!(
        config.get_property(section::PATH, "home"),
        Some(&"/home/user".to_string())
    );
    assert_eq!(
        config.get_property(section::BROWSER, "chrome"),
        Some(&"google-chrome".to_string())
    );
}

#[test]
fn test_get_property_missing_key_returns_none() {
    let config = make_populated_config();
    assert_eq!(config.get_property(section::PATH, "nonexistent"), None);
}

#[test]
fn test_get_property_invalid_section_returns_none() {
    let config = make_populated_config();
    assert_eq!(config.get_property("nonexistent", "home"), None);
}

// ========================================================================
// alias_exists
// ========================================================================

#[test]
fn test_alias_exists_in_path_returns_true() {
    let config = make_populated_config();
    assert!(config.alias_exists("home"), "alias should exist in path");
    assert!(config.alias_exists("proj"), "alias should exist in path");
}

#[test]
fn test_alias_exists_in_script_returns_true() {
    let config = make_populated_config();
    assert!(config.alias_exists("build"), "alias should exist in script");
}

#[test]
fn test_alias_exists_in_browser_returns_true() {
    let config = make_populated_config();
    assert!(
        config.alias_exists("chrome"),
        "alias should exist in browser"
    );
}

#[test]
fn test_alias_exists_in_editor_returns_true() {
    let config = make_populated_config();
    assert!(config.alias_exists("code"), "alias should exist in editor");
}

#[test]
fn test_alias_exists_not_found_returns_false() {
    let config = make_populated_config();
    assert!(!config.alias_exists("nonexistent"));
}

#[test]
fn test_alias_exists_empty_config_returns_false() {
    let config = YamlConfig::default();
    assert!(!config.alias_exists("anything"));
}

// ========================================================================
// get_path_by_alias
// ========================================================================

#[test]
fn test_get_path_by_alias_prioritizes_path_over_inner_url() {
    let mut config = YamlConfig::default();
    config.path.insert("dup".into(), "/from/path".into());
    config
        .inner_url
        .insert("dup".into(), "https://from.inner".into());
    config
        .outer_url
        .insert("dup".into(), "https://from.outer".into());
    assert_eq!(
        config.get_path_by_alias("dup"),
        Some(&"/from/path".to_string()),
        "path section should have highest priority"
    );
}

#[test]
fn test_get_path_by_alias_falls_back_to_inner_url() {
    let mut config = YamlConfig::default();
    config
        .inner_url
        .insert("only_inner".into(), "https://inner.example.com".into());
    config
        .outer_url
        .insert("only_inner".into(), "https://outer.example.com".into());
    assert_eq!(
        config.get_path_by_alias("only_inner"),
        Some(&"https://inner.example.com".to_string()),
        "should fall back to inner_url when not in path"
    );
}

#[test]
fn test_get_path_by_alias_falls_back_to_outer_url() {
    let mut config = YamlConfig::default();
    config
        .outer_url
        .insert("only_outer".into(), "https://outer.example.com".into());
    assert_eq!(
        config.get_path_by_alias("only_outer"),
        Some(&"https://outer.example.com".to_string()),
        "should fall back to outer_url"
    );
}

#[test]
fn test_get_path_by_alias_not_found_returns_none() {
    let config = make_populated_config();
    assert_eq!(config.get_path_by_alias("nonexistent"), None);
}

// ========================================================================
// collect_alias_envs
// ========================================================================

#[test]
fn test_collect_alias_envs_generates_correct_env_var_names() {
    let mut config = YamlConfig::default();
    config.path.insert("my-path".into(), "/some/path".into());
    config.path.insert("simple".into(), "/another".into());
    config
        .inner_url
        .insert("internal".into(), "https://internal".into());
    config
        .script
        .insert("build-all".into(), "cargo build --workspace".into());

    let envs = config.collect_alias_envs();
    let env_map: BTreeMap<&str, &str> =
        envs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    assert_eq!(
        env_map.get("J_MY_PATH"),
        Some(&"/some/path"),
        "hyphens should become underscores"
    );
    assert_eq!(env_map.get("J_SIMPLE"), Some(&"/another"));
    assert_eq!(
        env_map.get("J_INTERNAL"),
        Some(&"https://internal"),
        "inner_url aliases should be included"
    );
    assert_eq!(
        env_map.get("J_BUILD_ALL"),
        Some(&"cargo build --workspace"),
        "script aliases should be included"
    );
}

#[test]
fn test_collect_alias_envs_dedup_by_section_priority() {
    let mut config = YamlConfig::default();
    config.path.insert("dup".into(), "path-value".into());
    config.inner_url.insert("dup".into(), "inner-value".into());
    config.outer_url.insert("dup".into(), "outer-value".into());
    config.script.insert("dup".into(), "script-value".into());

    let envs = config.collect_alias_envs();
    let dup_count = envs.iter().filter(|(k, _)| k == "J_DUP").count();
    assert_eq!(dup_count, 1, "duplicate alias should appear only once");
    let dup_entry = envs
        .iter()
        .find(|(k, _)| k == "J_DUP")
        .expect("J_DUP should exist");
    assert_eq!(
        dup_entry.1, "path-value",
        "path section has highest priority"
    );
}

#[test]
fn test_collect_alias_envs_empty_config_returns_empty_vec() {
    let config = YamlConfig::default();
    let envs = config.collect_alias_envs();
    assert!(envs.is_empty(), "empty config should produce no envs");
}

#[test]
fn test_collect_alias_envs_only_covers_path_inner_outer_script() {
    let mut config = YamlConfig::default();
    config.path.insert("p".into(), "/p".into());
    config.inner_url.insert("iu".into(), "https://iu".into());
    config.outer_url.insert("ou".into(), "https://ou".into());
    config.script.insert("sc".into(), "echo hi".into());

    let envs = config.collect_alias_envs();
    let keys: Vec<&str> = envs.iter().map(|(k, _)| k.as_str()).collect();
    assert!(keys.contains(&"J_P"), "path alias should be present");
    assert!(keys.contains(&"J_IU"), "inner_url alias should be present");
    assert!(keys.contains(&"J_OU"), "outer_url alias should be present");
    assert!(keys.contains(&"J_SC"), "script alias should be present");
    // editor, browser, vpn should NOT be in envs
    assert!(
        keys.iter()
            .all(|k| !k.starts_with("J_CODE") && !k.starts_with("J_CHROME")),
        "non-alias sections should not appear"
    );
}
