//! 主题名称定义（不含 ratatui 依赖）
//!
//! Theme 结构体（含 ratatui Color）留在 j-cli 的 theme.rs 中。

use serde::{Deserialize, Serialize};

/// 主题名称枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ThemeName {
    #[serde(rename = "dark")]
    Dark,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "midnight")]
    #[default]
    Midnight,
    #[serde(rename = "nord")]
    Nord,
    #[serde(rename = "monokai")]
    Monokai,
    #[serde(rename = "anthropic_light")]
    AnthropicLight,
    #[serde(rename = "anthropic_dark")]
    AnthropicDark,
}

impl std::str::FromStr for ThemeName {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dark" => Ok(ThemeName::Dark),
            "light" => Ok(ThemeName::Light),
            "midnight" => Ok(ThemeName::Midnight),
            "nord" => Ok(ThemeName::Nord),
            "monokai" => Ok(ThemeName::Monokai),
            "anthropic_light" => Ok(ThemeName::AnthropicLight),
            "anthropic_dark" => Ok(ThemeName::AnthropicDark),
            _ => Ok(ThemeName::default()),
        }
    }
}

impl std::fmt::Display for ThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeName::Dark => write!(f, "dark"),
            ThemeName::Light => write!(f, "light"),
            ThemeName::Midnight => write!(f, "midnight"),
            ThemeName::Nord => write!(f, "nord"),
            ThemeName::Monokai => write!(f, "monokai"),
            ThemeName::AnthropicLight => write!(f, "anthropic_light"),
            ThemeName::AnthropicDark => write!(f, "anthropic_dark"),
        }
    }
}

impl ThemeName {
    /// 获取所有主题名称列表
    pub fn all() -> &'static [ThemeName] {
        &[
            ThemeName::Dark,
            ThemeName::Light,
            ThemeName::Midnight,
            ThemeName::Nord,
            ThemeName::Monokai,
            ThemeName::AnthropicLight,
            ThemeName::AnthropicDark,
        ]
    }

    /// 切换到下一个主题
    pub fn next(&self) -> ThemeName {
        match self {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::Midnight,
            ThemeName::Midnight => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Monokai,
            ThemeName::Monokai => ThemeName::AnthropicLight,
            ThemeName::AnthropicLight => ThemeName::AnthropicDark,
            ThemeName::AnthropicDark => ThemeName::Dark,
        }
    }

    /// 显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            ThemeName::Dark => "Dark",
            ThemeName::Light => "Light",
            ThemeName::Midnight => "Midnight（默认）",
            ThemeName::Nord => "Nord",
            ThemeName::Monokai => "Monokai",
            ThemeName::AnthropicLight => "Anthropic Light（米白赭陶）",
            ThemeName::AnthropicDark => "Anthropic Dark（深夜月蓝）",
        }
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> ThemeName {
        s.parse().unwrap_or_default()
    }

    /// 转为字符串
    pub fn to_str(&self) -> &'static str {
        match self {
            ThemeName::Dark => "dark",
            ThemeName::Light => "light",
            ThemeName::Midnight => "midnight",
            ThemeName::Nord => "nord",
            ThemeName::Monokai => "monokai",
            ThemeName::AnthropicLight => "anthropic_light",
            ThemeName::AnthropicDark => "anthropic_dark",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_lowercase() {
        assert_eq!(ThemeName::parse("dark"), ThemeName::Dark);
        assert_eq!(ThemeName::parse("light"), ThemeName::Light);
        assert_eq!(ThemeName::parse("midnight"), ThemeName::Midnight);
        assert_eq!(ThemeName::parse("nord"), ThemeName::Nord);
        assert_eq!(ThemeName::parse("monokai"), ThemeName::Monokai);
        assert_eq!(
            ThemeName::parse("anthropic_light"),
            ThemeName::AnthropicLight
        );
        assert_eq!(ThemeName::parse("anthropic_dark"), ThemeName::AnthropicDark);
    }

    #[test]
    fn test_parse_case_insensitive() {
        assert_eq!(ThemeName::parse("DARK"), ThemeName::Dark);
        assert_eq!(ThemeName::parse("Midnight"), ThemeName::Midnight);
        assert_eq!(ThemeName::parse("NORD"), ThemeName::Nord);
    }

    #[test]
    fn test_parse_invalid_falls_back_to_default() {
        assert_eq!(ThemeName::parse("invalid"), ThemeName::default());
        assert_eq!(ThemeName::parse(""), ThemeName::default());
    }

    #[test]
    fn test_to_str_round_trip() {
        for name in ThemeName::all() {
            let s = name.to_str();
            let parsed = ThemeName::parse(s);
            assert_eq!(parsed, *name, "round-trip failed for {name:?}");
        }
    }

    #[test]
    fn test_display_matches_to_str() {
        for name in ThemeName::all() {
            assert_eq!(name.to_string(), name.to_str());
        }
    }

    #[test]
    fn test_all_contains_all_variants() {
        let all = ThemeName::all();
        assert_eq!(all.len(), 7, "should have 7 theme variants");
        // Verify no duplicates
        let mut seen = std::collections::HashSet::new();
        for name in all {
            assert!(seen.insert(name.to_str()), "duplicate: {name:?}");
        }
    }

    #[test]
    fn test_default_is_midnight() {
        assert_eq!(ThemeName::default(), ThemeName::Midnight);
    }
}
