//! 编辑器独立主题（解耦 chat::Theme）
//!
//! 定义编辑器渲染所需的所有样式字段，
//! 使 editor_core 模块不依赖 chat 子系统。

use ratatui::style::Color;
use std::sync::OnceLock;

static GLOBAL_BORDER_STYLE: OnceLock<BorderStyle> = OnceLock::new();

/// 初始化全局边框样式（由 j-cli 在启动时调用）
pub fn init_border_style(style: &str) {
    let border_style = BorderStyle::from_config(style);
    let _ = GLOBAL_BORDER_STYLE.set(border_style);
}

/// 获取当前生效的边框样式
pub fn current_border_style() -> BorderStyle {
    GLOBAL_BORDER_STYLE.get().copied().unwrap_or_default()
}

// 代码块边框样式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    /// 圆角边框：╭╮╰╯
    #[default]
    Rounded,
    /// 直角边框：┌┐└┘
    Plain,
}

impl BorderStyle {
    /// 从配置字符串解析边框样式
    pub fn from_config(s: &str) -> Self {
        match s {
            "plain" => BorderStyle::Plain,
            _ => BorderStyle::Rounded,
        }
    }

    /// 获取左上角字符
    pub const fn top_left(&self) -> &'static str {
        match self {
            BorderStyle::Rounded => "╭",
            BorderStyle::Plain => "┌",
        }
    }

    /// 获取右上角字符
    pub const fn top_right(&self) -> &'static str {
        match self {
            BorderStyle::Rounded => "╮",
            BorderStyle::Plain => "┐",
        }
    }

    /// 获取左下角字符
    pub const fn bottom_left(&self) -> &'static str {
        match self {
            BorderStyle::Rounded => "╰",
            BorderStyle::Plain => "└",
        }
    }

    /// 获取垂直边框字符
    pub const fn vertical(&self) -> &'static str {
        "│"
    }

    /// 获取水平边框字符
    pub const fn horizontal(&self) -> &'static str {
        "─"
    }

    /// 获取右下角字符
    pub const fn bottom_right(&self) -> &'static str {
        match self {
            BorderStyle::Rounded => "╯",
            BorderStyle::Plain => "┘",
        }
    }
}

/// 编辑器主题
#[derive(Debug, Clone, PartialEq)]
pub struct EditorTheme {
    // ===== 全局背景 =====
    pub bg_primary: Color,
    pub bg_input: Color,
    pub code_bg: Color,

    // ===== 光标 =====
    pub cursor_fg: Color,
    pub cursor_bg: Color,

    // ===== 文本 =====
    pub text_normal: Color,
    pub text_dim: Color,
    pub text_bold: Color,
    pub text_very_dim: Color,
    pub text_white: Color,

    // ===== 分隔线 =====
    pub separator: Color,

    // ===== Markdown =====
    pub md_h1: Color,
    pub md_h2: Color,
    pub md_h3: Color,
    pub md_h4: Color,
    pub md_heading_sep: Color,
    pub md_link: Color,
    pub md_list_bullet: Color,
    pub md_blockquote_bar: Color,
    pub md_blockquote_bg: Color,
    pub md_blockquote_text: Color,
    pub md_inline_code_fg: Color,
    pub md_inline_code_bg: Color,
    pub md_rule: Color,

    // ===== 代码块 =====
    pub code_border: Color,
    /// 代码块边框样式：圆角或直角
    pub code_border_style: BorderStyle,

    // ===== 表格 =====
    pub table_header: Color,
    pub table_body: Color,

    // ===== 标签 =====
    pub label_ai: Color,

    // ===== 配置界面 =====
    pub config_pointer: Color,
    pub config_label_selected: Color,
    pub config_label: Color,
    pub config_value: Color,
    pub config_edit_bg: Color,
    pub config_tab_active_bg: Color,
    pub config_tab_active_fg: Color,
    pub config_tab_inactive: Color,
    pub config_toggle_on: Color,
    pub config_toggle_off: Color,
    pub config_dim: Color,

    // ===== 帮助界面 =====
    pub help_title: Color,
    pub help_key: Color,
    pub help_desc: Color,

    // ===== 代码高亮 =====
    pub code_default: Color,
    pub code_keyword: Color,
    pub code_string: Color,
    pub code_comment: Color,
    pub code_number: Color,
    pub code_type: Color,
    pub code_primitive: Color,
    pub code_macro: Color,
    pub code_lifetime: Color,
    pub code_attribute: Color,
    pub code_shell_var: Color,
}

/// 语法高亮函数类型
pub type HighlightFn = fn(&str, &str, &EditorTheme) -> Vec<ratatui::text::Span<'static>>;

// ---------------------------------------------------------------------------
// MdStyle trait 实现：让共享渲染层可以直接使用 EditorTheme
// ---------------------------------------------------------------------------

impl crate::markdown::theme::MdStyle for EditorTheme {
    fn text_normal(&self) -> Color {
        self.text_normal
    }
    fn text_bold(&self) -> Color {
        self.text_bold
    }
    fn text_dim(&self) -> Color {
        self.text_dim
    }

    fn md_h1(&self) -> Color {
        self.md_h1
    }
    fn md_h2(&self) -> Color {
        self.md_h2
    }
    fn md_h3(&self) -> Color {
        self.md_h3
    }
    fn md_h4(&self) -> Color {
        self.md_h4
    }
    fn md_heading_sep(&self) -> Color {
        self.md_heading_sep
    }

    fn md_link(&self) -> Color {
        self.md_link
    }
    fn md_inline_code_fg(&self) -> Color {
        self.md_inline_code_fg
    }

    fn bg_primary(&self) -> Color {
        self.bg_primary
    }

    fn md_blockquote_bar(&self) -> Color {
        self.md_blockquote_bar
    }
    fn md_blockquote_bg(&self) -> Color {
        self.md_blockquote_bg
    }
    fn md_blockquote_text(&self) -> Color {
        self.md_blockquote_text
    }

    fn md_list_bullet(&self) -> Color {
        self.md_list_bullet
    }
    fn md_rule(&self) -> Color {
        self.md_rule
    }

    fn table_header(&self) -> Color {
        self.table_header
    }
    fn table_body(&self) -> Color {
        self.table_body
    }

    fn code_syntax_theme(&self) -> EditorTheme {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_border_style_default_is_rounded() {
        assert_eq!(BorderStyle::default(), BorderStyle::Rounded);
    }

    #[test]
    fn test_border_style_from_config_plain() {
        assert_eq!(BorderStyle::from_config("plain"), BorderStyle::Plain);
    }

    #[test]
    fn test_border_style_from_config_unknown_falls_back_to_rounded() {
        assert_eq!(BorderStyle::from_config("rounded"), BorderStyle::Rounded);
        assert_eq!(BorderStyle::from_config(""), BorderStyle::Rounded);
        assert_eq!(BorderStyle::from_config("garbage"), BorderStyle::Rounded);
    }

    #[test]
    fn test_border_style_corners_rounded() {
        assert_eq!(BorderStyle::Rounded.top_left(), "╭");
        assert_eq!(BorderStyle::Rounded.top_right(), "╮");
        assert_eq!(BorderStyle::Rounded.bottom_left(), "╰");
        assert_eq!(BorderStyle::Rounded.bottom_right(), "╯");
    }

    #[test]
    fn test_border_style_corners_plain() {
        assert_eq!(BorderStyle::Plain.top_left(), "┌");
        assert_eq!(BorderStyle::Plain.top_right(), "┐");
        assert_eq!(BorderStyle::Plain.bottom_left(), "└");
        assert_eq!(BorderStyle::Plain.bottom_right(), "┘");
    }

    #[test]
    fn test_border_style_vertical_and_horizontal() {
        assert_eq!(BorderStyle::Rounded.vertical(), "│");
        assert_eq!(BorderStyle::Rounded.horizontal(), "─");
        assert_eq!(BorderStyle::Plain.vertical(), "│");
        assert_eq!(BorderStyle::Plain.horizontal(), "─");
    }
}
