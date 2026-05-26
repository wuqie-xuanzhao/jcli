pub use super::yaml_config::*;
pub use crate::constants::{ALL_SECTIONS, REPORT_DEFAULT_FILE, config_key, section};

// ========================================================================
// Helpers
// ========================================================================

pub(super) fn make_populated_config() -> YamlConfig {
    let mut c = YamlConfig::default();
    c.path.insert("home".into(), "/home/user".into());
    c.path.insert("proj".into(), "/home/user/projects".into());
    c.inner_url
        .insert("gitlab".into(), "https://gitlab.internal.com".into());
    c.outer_url
        .insert("github".into(), "https://github.com".into());
    c.editor.insert("code".into(), "code".into());
    c.browser.insert("chrome".into(), "google-chrome".into());
    c.vpn.insert("office".into(), "vpn://office".into());
    c.script.insert("build".into(), "cargo build".into());
    c
}

mod paths;
mod properties;
mod serde;
