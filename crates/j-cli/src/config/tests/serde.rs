use super::*;

// ========================================================================
// Serde / YAML parsing
// ========================================================================

#[test]
fn test_serde_roundtrip_preserves_all_data() {
    let mut config = make_populated_config();
    config
        .setting
        .insert("search-engine".into(), "google".into());
    config.log.insert("mode".into(), "verbose".into());

    let yaml = serde_yaml::to_string(&config).expect("serialize");
    let restored: YamlConfig = serde_yaml::from_str(&yaml).expect("deserialize");

    assert_eq!(
        restored.path.get("home"),
        Some(&"/home/user".to_string()),
        "path.home should survive roundtrip"
    );
    assert_eq!(
        restored.path.get("proj"),
        Some(&"/home/user/projects".to_string()),
        "path.proj should survive roundtrip"
    );
    assert_eq!(
        restored.outer_url.get("github"),
        Some(&"https://github.com".to_string()),
        "outer_url.github should survive roundtrip"
    );
    assert_eq!(
        restored.setting.get("search-engine"),
        Some(&"google".to_string()),
        "setting.search-engine should survive roundtrip"
    );
    assert_eq!(
        restored.log.get("mode"),
        Some(&"verbose".to_string()),
        "log.mode should survive roundtrip"
    );
    assert!(restored.version.is_empty());
    assert!(restored.report.is_empty());
}

#[test]
fn test_deserialize_valid_yaml_populates_maps() {
    let yaml = r#"
path:
  home: /home/user
  projects: /home/user/projects
inner_url:
  gitlab: https://gitlab.internal.com
outer_url:
  github: https://github.com
editor:
  code: code
browser:
  chrome: google-chrome
"#;
    let config: YamlConfig = serde_yaml::from_str(yaml).expect("parse valid YAML");
    assert_eq!(
        config.path.get("home"),
        Some(&"/home/user".to_string()),
        "path.home"
    );
    assert_eq!(
        config.path.get("projects"),
        Some(&"/home/user/projects".to_string()),
        "path.projects"
    );
    assert_eq!(
        config.inner_url.get("gitlab"),
        Some(&"https://gitlab.internal.com".to_string()),
        "inner_url.gitlab"
    );
    assert_eq!(
        config.outer_url.get("github"),
        Some(&"https://github.com".to_string()),
        "outer_url.github"
    );
}

#[test]
fn test_deserialize_empty_yaml_yields_default() {
    let config: YamlConfig = serde_yaml::from_str("{}").expect("parse empty YAML");
    assert!(config.path.is_empty());
    assert!(config.inner_url.is_empty());
    assert!(config.outer_url.is_empty());
    assert!(config.editor.is_empty());
    assert!(config.browser.is_empty());
    assert!(config.vpn.is_empty());
    assert!(config.script.is_empty());
    assert!(config.version.is_empty());
    assert!(config.setting.is_empty());
    assert!(config.log.is_empty());
    assert!(config.report.is_empty());
}

#[test]
fn test_deserialize_missing_field_defaults_to_empty() {
    let yaml = r#"
path:
  home: /home/user
"#;
    let config: YamlConfig = serde_yaml::from_str(yaml).expect("parse partial YAML");
    assert_eq!(
        config.path.get("home"),
        Some(&"/home/user".to_string()),
        "path.home should be present"
    );
    assert!(config.inner_url.is_empty());
    assert!(config.outer_url.is_empty());
    assert!(config.editor.is_empty());
}

#[test]
fn test_deserialize_extra_top_level_key_goes_to_extra() {
    let yaml = r#"
path:
  home: /home/user
custom_tool:
  name: my-tool
  version: "1.0"
"#;
    let config: YamlConfig = serde_yaml::from_str(yaml).expect("parse YAML with extra key");
    assert_eq!(config.path.get("home"), Some(&"/home/user".to_string()));
    assert!(
        config.extra.contains_key("custom_tool"),
        "custom_tool should be captured in extra"
    );
}

#[test]
fn test_deserialize_invalid_yaml_returns_error() {
    let yaml = "path: [unclosed\n  mapping: yes";
    let result: Result<YamlConfig, _> = serde_yaml::from_str(yaml);
    assert!(result.is_err(), "invalid YAML should produce an error");
}

#[test]
fn test_deserialize_null_value_does_not_crash() {
    let yaml = r#"
path:
  home: /home/user
  empty_key: ~
"#;
    let config: YamlConfig = serde_yaml::from_str(yaml).expect("parse YAML with null value");
    assert_eq!(config.path.get("home"), Some(&"/home/user".to_string()));
}

#[test]
fn test_deserialize_special_characters_in_keys_and_values() {
    let yaml = r#"
path:
  "my-key": "C:\\Users\\name"
  "a.b.c": "value with spaces and 中文"
  "underscore_key": "value\nwith\nnewlines"
"#;
    let config: YamlConfig = serde_yaml::from_str(yaml).expect("parse YAML with special chars");
    assert_eq!(
        config.path.get("my-key"),
        Some(&"C:\\Users\\name".to_string())
    );
    assert_eq!(
        config.path.get("a.b.c"),
        Some(&"value with spaces and 中文".to_string())
    );
    assert_eq!(
        config.path.get("underscore_key"),
        Some(&"value\nwith\nnewlines".to_string())
    );
}

#[test]
fn test_deserialize_large_config() {
    let mut yaml_parts = vec!["path:".to_string()];
    for i in 0..200 {
        yaml_parts.push(format!("  key_{:03}: /path/to/{}", i, i));
    }
    let yaml = yaml_parts.join("\n");
    let config: YamlConfig = serde_yaml::from_str(&yaml).expect("parse large YAML");
    assert_eq!(config.path.len(), 200, "should have 200 path entries");
    assert_eq!(config.path.get("key_000"), Some(&"/path/to/0".to_string()));
    assert_eq!(
        config.path.get("key_199"),
        Some(&"/path/to/199".to_string())
    );
}

// ========================================================================
// Default / construction
// ========================================================================

#[test]
fn test_default_trait_yields_empty_maps() {
    let config = YamlConfig::default();
    assert!(config.path.is_empty());
    assert!(config.inner_url.is_empty());
    assert!(config.outer_url.is_empty());
    assert!(config.editor.is_empty());
    assert!(config.browser.is_empty());
    assert!(config.vpn.is_empty());
    assert!(config.script.is_empty());
    assert!(config.version.is_empty());
    assert!(config.setting.is_empty());
    assert!(config.log.is_empty());
    assert!(config.report.is_empty());
    assert!(config.extra.is_empty());
}

// ========================================================================
// all_section_names
// ========================================================================

#[test]
fn test_all_section_names_matches_all_sections_constant() {
    let config = YamlConfig::default();
    let names = config.all_section_names();
    assert_eq!(names.len(), ALL_SECTIONS.len());
    for (i, &name) in names.iter().enumerate() {
        assert_eq!(name, ALL_SECTIONS[i]);
    }
}

// ========================================================================
// Clone
// ========================================================================

#[test]
fn test_clone_produces_independent_copy() {
    let config = make_populated_config();
    let mut cloned = config.clone();

    cloned
        .path
        .insert("clone-only".into(), "/clone/value".into());
    cloned.path.remove("home");

    assert!(
        config.path.contains_key("home"),
        "original should still have 'home'"
    );
    assert!(
        !config.path.contains_key("clone-only"),
        "original should not have clone-only"
    );
    assert!(
        cloned.path.contains_key("clone-only"),
        "clone should have clone-only"
    );
    assert!(
        !cloned.path.contains_key("home"),
        "clone should not have home"
    );
}
