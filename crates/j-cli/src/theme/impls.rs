use j_tui::editor_core::theme::BorderStyle;
use ratatui::style::Color;

use crate::assets::Assets;
use crate::theme::types::ThemeName;

use super::Theme;

impl Theme {
    /// 根据主题名称从嵌入资源加载主题
    pub fn from_name(name: &ThemeName) -> Self {
        let filename = format!("themes/{}.json", name.to_str());
        let theme = match Self::load_from_assets(&filename) {
            Ok(theme) => theme,
            Err(_) => {
                // 主题加载失败时静默回退，不输出日志（无 config 上下文）
                // 回退到 midnight（如果连默认都加载失败则用 terminal）
                if *name != ThemeName::Midnight {
                    match Self::load_from_assets("themes/midnight.json") {
                        Ok(t) => t,
                        Err(_) => Self::terminal_fallback(),
                    }
                } else {
                    Self::terminal_fallback()
                }
            }
        };
        // 注入全局边框样式配置
        theme.with_border_style(j_tui::editor_core::theme::current_border_style())
    }

    /// 设置边框样式（从全局配置注入）
    fn with_border_style(mut self, style: BorderStyle) -> Self {
        self.code_border_style = style;
        self
    }

    /// 从 Assets 加载并解析主题 JSON
    fn load_from_assets(path: &str) -> Result<Self, String> {
        let asset = Assets::get(path).ok_or_else(|| format!("asset not found: {path}"))?;
        let json_str = std::str::from_utf8(&asset.data).map_err(|e| e.to_string())?;
        super::parse::parse_theme_json(json_str, path)
    }

    /// 终端原生主题：使用标准 ANSI 颜色，适合非 AI 模式下的终端输出
    /// 颜色跟随终端自身配色方案，不依赖 agent 配置
    pub fn terminal() -> Self {
        Self::terminal_fallback()
    }

    /// 终端回退主题：使用标准 ANSI 颜色，不依赖外部文件
    fn terminal_fallback() -> Self {
        Self {
            bg_primary: Color::Reset,
            bg_title: Color::Reset,
            bg_input: Color::Reset,
            bg_panel: Color::Reset,
            border_title: Color::DarkGray,
            border_message: Color::DarkGray,
            border_input: Color::DarkGray,
            border_input_loading: Color::DarkGray,
            border_config: Color::DarkGray,
            separator: Color::DarkGray,
            bubble_ai: Color::Reset,
            bubble_ai_selected: Color::Reset,
            bubble_user: Color::Reset,
            bubble_user_selected: Color::Reset,
            label_ai: Color::Reset,
            label_user: Color::Reset,
            label_selected: Color::Reset,
            text_normal: Color::Reset,
            text_bold: Color::White,
            text_dim: Color::DarkGray,
            text_very_dim: Color::DarkGray,
            text_white: Color::White,
            text_system: Color::DarkGray,
            title_icon: Color::Reset,
            title_separator: Color::DarkGray,
            title_model: Color::Reset,
            title_count: Color::Reset,
            title_loading: Color::Reset,
            input_prompt: Color::Reset,
            input_prompt_loading: Color::Reset,
            cursor_fg: Color::Reset,
            cursor_bg: Color::Reset,
            hint_key_fg: Color::Reset,
            hint_key_bg: Color::Reset,
            hint_desc: Color::Reset,
            hint_separator: Color::DarkGray,
            toast_success_border: Color::Green,
            toast_success_bg: Color::Reset,
            toast_success_text: Color::LightGreen,
            toast_error_border: Color::Red,
            toast_error_bg: Color::Reset,
            toast_error_text: Color::LightRed,
            tool_confirm_border: Color::Cyan,
            tool_confirm_bg: Color::Blue,
            tool_confirm_title: Color::Yellow,
            tool_confirm_name: Color::Yellow,
            tool_confirm_text: Color::White,
            tool_confirm_label: Color::White,
            tool_confirm_hint: Color::Yellow,
            welcome_border: Color::DarkGray,
            welcome_text: Color::Reset,
            welcome_hint: Color::DarkGray,
            welcome_quote: Color::DarkGray,
            welcome_palette: 5,
            model_sel_border: Color::DarkGray,
            model_sel_title: Color::Reset,
            model_sel_active: Color::LightGreen,
            model_sel_inactive: Color::Reset,
            model_sel_highlight_bg: Color::Reset,
            model_sel_highlight_fg: Color::Yellow,
            config_title: Color::LightCyan,
            config_section: Color::LightGreen,
            config_pointer: Color::Yellow,
            config_label_selected: Color::Yellow,
            config_label: Color::DarkGray,
            config_value: Color::Reset,
            config_edit_bg: Color::Reset,
            config_tab_active_bg: Color::LightCyan,
            config_tab_active_fg: Color::Reset,
            config_tab_inactive: Color::DarkGray,
            config_hint_key: Color::Yellow,
            config_hint_desc: Color::DarkGray,
            config_toggle_on: Color::LightGreen,
            config_toggle_off: Color::Red,
            config_dim: Color::DarkGray,
            config_api_key: Color::DarkGray,
            md_h1: Color::LightCyan,
            md_h2: Color::Cyan,
            md_h3: Color::LightBlue,
            md_h4: Color::Blue,
            md_heading_sep: Color::DarkGray,
            md_inline_code_fg: Color::LightYellow,
            md_inline_code_bg: Color::Reset,
            md_list_bullet: Color::LightGreen,
            md_blockquote_bar: Color::Cyan,
            md_blockquote_text: Color::Gray,
            md_blockquote_bg: Color::Reset,
            md_rule: Color::DarkGray,
            md_link: Color::LightBlue,
            code_border: Color::DarkGray,
            code_border_style: BorderStyle::default(),
            code_bg: Color::Reset,
            code_default: Color::Reset,
            code_keyword: Color::LightMagenta,
            code_string: Color::LightGreen,
            code_comment: Color::DarkGray,
            code_number: Color::LightYellow,
            code_type: Color::LightYellow,
            code_primitive: Color::LightCyan,
            code_macro: Color::LightBlue,
            code_attribute: Color::LightCyan,
            code_lifetime: Color::LightYellow,
            code_shell_var: Color::LightCyan,
            table_border: Color::DarkGray,
            table_header: Color::LightCyan,
            table_body: Color::Reset,
            help_title: Color::LightCyan,
            help_key: Color::Yellow,
            help_desc: Color::Reset,
            help_path: Color::DarkGray,
            help_bg: Color::Reset,
            diff_add: Color::LightGreen,
            diff_del: Color::LightRed,
            diff_header: Color::LightCyan,
        }
    }
}

// ---------------------------------------------------------------------------
// MdStyle trait 实现：让 Theme 可直接用于 j-tui 的 Markdown 渲染
// ---------------------------------------------------------------------------

impl j_tui::markdown::theme::MdStyle for Theme {
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
    fn code_syntax_theme(&self) -> j_tui::editor_core::EditorTheme {
        j_tui::editor_core::EditorTheme::from(self)
    }
}
