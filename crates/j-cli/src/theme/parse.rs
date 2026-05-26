use j_tui::editor_core::theme::BorderStyle;
use ratatui::style::Color;
use serde::Deserialize;

use super::Theme;

// ===== JSON 反序列化中间结构 =====

/// JSON 主题文件的中间表示（所有字段为可反序列化类型）
#[derive(Deserialize)]
pub(super) struct ThemeJson {
    bg_primary: ColorValue,
    bg_title: ColorValue,
    bg_input: ColorValue,
    bg_panel: ColorValue,
    border_title: ColorValue,
    border_message: ColorValue,
    border_input: ColorValue,
    border_input_loading: ColorValue,
    border_config: ColorValue,
    separator: ColorValue,
    bubble_ai: ColorValue,
    bubble_ai_selected: ColorValue,
    bubble_user: ColorValue,
    bubble_user_selected: ColorValue,
    label_ai: ColorValue,
    label_user: ColorValue,
    label_selected: ColorValue,
    text_normal: ColorValue,
    text_bold: ColorValue,
    text_dim: ColorValue,
    text_very_dim: ColorValue,
    text_white: ColorValue,
    text_system: ColorValue,
    title_icon: ColorValue,
    title_separator: ColorValue,
    title_model: ColorValue,
    title_count: ColorValue,
    title_loading: ColorValue,
    input_prompt: ColorValue,
    input_prompt_loading: ColorValue,
    cursor_fg: ColorValue,
    cursor_bg: ColorValue,
    hint_key_fg: ColorValue,
    hint_key_bg: ColorValue,
    hint_desc: ColorValue,
    hint_separator: ColorValue,
    toast_success_border: ColorValue,
    toast_success_bg: ColorValue,
    toast_success_text: ColorValue,
    toast_error_border: ColorValue,
    toast_error_bg: ColorValue,
    toast_error_text: ColorValue,
    tool_confirm_border: ColorValue,
    tool_confirm_bg: ColorValue,
    tool_confirm_title: ColorValue,
    tool_confirm_name: ColorValue,
    tool_confirm_text: ColorValue,
    tool_confirm_label: ColorValue,
    tool_confirm_hint: ColorValue,
    welcome_border: ColorValue,
    welcome_text: ColorValue,
    welcome_hint: ColorValue,
    welcome_quote: ColorValue,
    welcome_palette: u8,
    model_sel_border: ColorValue,
    model_sel_title: ColorValue,
    model_sel_active: ColorValue,
    model_sel_inactive: ColorValue,
    model_sel_highlight_bg: ColorValue,
    model_sel_highlight_fg: ColorValue,
    config_title: ColorValue,
    config_section: ColorValue,
    config_pointer: ColorValue,
    config_label_selected: ColorValue,
    config_label: ColorValue,
    config_value: ColorValue,
    config_edit_bg: ColorValue,
    config_tab_active_bg: ColorValue,
    config_tab_active_fg: ColorValue,
    config_tab_inactive: ColorValue,
    config_hint_key: ColorValue,
    config_hint_desc: ColorValue,
    config_toggle_on: ColorValue,
    config_toggle_off: ColorValue,
    config_dim: ColorValue,
    config_api_key: ColorValue,
    md_h1: ColorValue,
    md_h2: ColorValue,
    md_h3: ColorValue,
    md_h4: ColorValue,
    md_heading_sep: ColorValue,
    md_inline_code_fg: ColorValue,
    md_inline_code_bg: ColorValue,
    md_list_bullet: ColorValue,
    md_blockquote_bar: ColorValue,
    md_blockquote_text: ColorValue,
    md_blockquote_bg: ColorValue,
    md_rule: ColorValue,
    md_link: ColorValue,
    code_border: ColorValue,
    code_bg: ColorValue,
    code_default: ColorValue,
    code_keyword: ColorValue,
    code_string: ColorValue,
    code_comment: ColorValue,
    code_number: ColorValue,
    code_type: ColorValue,
    code_primitive: ColorValue,
    code_macro: ColorValue,
    code_attribute: ColorValue,
    code_lifetime: ColorValue,
    code_shell_var: ColorValue,
    table_border: ColorValue,
    table_header: ColorValue,
    table_body: ColorValue,
    help_title: ColorValue,
    help_key: ColorValue,
    help_desc: ColorValue,
    help_path: ColorValue,
    help_bg: ColorValue,
    diff_add: ColorValue,
    diff_del: ColorValue,
    diff_header: ColorValue,
}

impl From<ThemeJson> for Theme {
    fn from(j: ThemeJson) -> Self {
        Self {
            bg_primary: j.bg_primary.0,
            bg_title: j.bg_title.0,
            bg_input: j.bg_input.0,
            bg_panel: j.bg_panel.0,
            border_title: j.border_title.0,
            border_message: j.border_message.0,
            border_input: j.border_input.0,
            border_input_loading: j.border_input_loading.0,
            border_config: j.border_config.0,
            separator: j.separator.0,
            bubble_ai: j.bubble_ai.0,
            bubble_ai_selected: j.bubble_ai_selected.0,
            bubble_user: j.bubble_user.0,
            bubble_user_selected: j.bubble_user_selected.0,
            label_ai: j.label_ai.0,
            label_user: j.label_user.0,
            label_selected: j.label_selected.0,
            text_normal: j.text_normal.0,
            text_bold: j.text_bold.0,
            text_dim: j.text_dim.0,
            text_very_dim: j.text_very_dim.0,
            text_white: j.text_white.0,
            text_system: j.text_system.0,
            title_icon: j.title_icon.0,
            title_separator: j.title_separator.0,
            title_model: j.title_model.0,
            title_count: j.title_count.0,
            title_loading: j.title_loading.0,
            input_prompt: j.input_prompt.0,
            input_prompt_loading: j.input_prompt_loading.0,
            cursor_fg: j.cursor_fg.0,
            cursor_bg: j.cursor_bg.0,
            hint_key_fg: j.hint_key_fg.0,
            hint_key_bg: j.hint_key_bg.0,
            hint_desc: j.hint_desc.0,
            hint_separator: j.hint_separator.0,
            toast_success_border: j.toast_success_border.0,
            toast_success_bg: j.toast_success_bg.0,
            toast_success_text: j.toast_success_text.0,
            toast_error_border: j.toast_error_border.0,
            toast_error_bg: j.toast_error_bg.0,
            toast_error_text: j.toast_error_text.0,
            tool_confirm_border: j.tool_confirm_border.0,
            tool_confirm_bg: j.tool_confirm_bg.0,
            tool_confirm_title: j.tool_confirm_title.0,
            tool_confirm_name: j.tool_confirm_name.0,
            tool_confirm_text: j.tool_confirm_text.0,
            tool_confirm_label: j.tool_confirm_label.0,
            tool_confirm_hint: j.tool_confirm_hint.0,
            welcome_border: j.welcome_border.0,
            welcome_text: j.welcome_text.0,
            welcome_hint: j.welcome_hint.0,
            welcome_quote: j.welcome_quote.0,
            welcome_palette: j.welcome_palette,
            model_sel_border: j.model_sel_border.0,
            model_sel_title: j.model_sel_title.0,
            model_sel_active: j.model_sel_active.0,
            model_sel_inactive: j.model_sel_inactive.0,
            model_sel_highlight_bg: j.model_sel_highlight_bg.0,
            model_sel_highlight_fg: j.model_sel_highlight_fg.0,
            config_title: j.config_title.0,
            config_section: j.config_section.0,
            config_pointer: j.config_pointer.0,
            config_label_selected: j.config_label_selected.0,
            config_label: j.config_label.0,
            config_value: j.config_value.0,
            config_edit_bg: j.config_edit_bg.0,
            config_tab_active_bg: j.config_tab_active_bg.0,
            config_tab_active_fg: j.config_tab_active_fg.0,
            config_tab_inactive: j.config_tab_inactive.0,
            config_hint_key: j.config_hint_key.0,
            config_hint_desc: j.config_hint_desc.0,
            config_toggle_on: j.config_toggle_on.0,
            config_toggle_off: j.config_toggle_off.0,
            config_dim: j.config_dim.0,
            config_api_key: j.config_api_key.0,
            md_h1: j.md_h1.0,
            md_h2: j.md_h2.0,
            md_h3: j.md_h3.0,
            md_h4: j.md_h4.0,
            md_heading_sep: j.md_heading_sep.0,
            md_inline_code_fg: j.md_inline_code_fg.0,
            md_inline_code_bg: j.md_inline_code_bg.0,
            md_list_bullet: j.md_list_bullet.0,
            md_blockquote_bar: j.md_blockquote_bar.0,
            md_blockquote_text: j.md_blockquote_text.0,
            md_blockquote_bg: j.md_blockquote_bg.0,
            md_rule: j.md_rule.0,
            md_link: j.md_link.0,
            code_border: j.code_border.0,
            code_border_style: BorderStyle::default(),
            code_bg: j.code_bg.0,
            code_default: j.code_default.0,
            code_keyword: j.code_keyword.0,
            code_string: j.code_string.0,
            code_comment: j.code_comment.0,
            code_number: j.code_number.0,
            code_type: j.code_type.0,
            code_primitive: j.code_primitive.0,
            code_macro: j.code_macro.0,
            code_attribute: j.code_attribute.0,
            code_lifetime: j.code_lifetime.0,
            code_shell_var: j.code_shell_var.0,
            table_border: j.table_border.0,
            table_header: j.table_header.0,
            table_body: j.table_body.0,
            help_title: j.help_title.0,
            help_key: j.help_key.0,
            help_desc: j.help_desc.0,
            help_path: j.help_path.0,
            help_bg: j.help_bg.0,
            diff_add: j.diff_add.0,
            diff_del: j.diff_del.0,
            diff_header: j.diff_header.0,
        }
    }
}

/// 颜色值的 JSON 反序列化包装
///
/// 支持两种形态：
/// 1. **简单形态（字符串）**：与早期 schema 完全兼容
///    - `"#rrggbb"` → `Color::Rgb(r, g, b)`
///    - `"reset"` / `"white"` / `"dark_gray"` 等 → 对应 ANSI 命名色
///
/// 2. **多级形态（对象）**：可对关键色显式锁定 fallback，避免运行时最近邻误判
///    ```json
///    { "rgb": "#d7263d", "ansi256": 160, "ansi16": "red" }
///    ```
///    - `ansi16` 字段在全局 [`crate::util::color_adapt::ColorLevel::Ansi16`] 模式下命中
///    - `ansi256` 在 Ansi256 模式下命中
///    - 否则回落到 `rgb`（之后由 [`crate::util::color_adapt::degrade`] 在使用时降级）
#[derive(Deserialize)]
#[serde(untagged)]
enum RawColor {
    Simple(String),
    Multi {
        #[serde(default)]
        rgb: Option<String>,
        #[serde(default)]
        ansi256: Option<u8>,
        #[serde(default)]
        ansi16: Option<String>,
    },
}

#[derive(Deserialize)]
#[serde(try_from = "RawColor")]
struct ColorValue(Color);

impl TryFrom<RawColor> for ColorValue {
    type Error = String;

    fn try_from(raw: RawColor) -> Result<Self, Self::Error> {
        let color = match raw {
            RawColor::Simple(s) => parse_color(&s)?,
            RawColor::Multi {
                rgb,
                ansi256,
                ansi16,
            } => resolve_multi_color(rgb.as_deref(), ansi256, ansi16.as_deref())?,
        };
        Ok(ColorValue(color))
    }
}

/// 在多级颜色对象中按当前 [`crate::util::color_adapt::ColorLevel`] 选取最优值。
///
/// 优先级：当前色阶的精确字段 > rgb > 其他色阶兜底。
///
/// # 为什么这个函数是 TUI 与 `color_mode` 的**唯一**桥梁
/// chat TUI / todo TUI 走 ratatui+crossterm 渲染，**不会调用** [`crate::util::color_adapt::degrade`]
/// 或 [`crate::util::color_adapt::apply_fg`]，所以运行时的色阶降级对 TUI 无效。
/// 但 ColorValue 反序列化时（即 [`Theme::from_name`] 加载 JSON 那一刻）会调用 `current()`，
/// 让对象形态的 JSON 字段在那时就解析成最终的 ratatui::Color 存进 Theme。
/// TUI 之后拿到的就是这个已经"定型"的颜色值——色阶在 JSON 加载阶段一次性敲定，
/// 不需要 TUI 在每次渲染时重做降级。
///
/// 因此：内置主题全是字符串形态 → 走 `RawColor::Simple` → 不读 ColorLevel → TUI 零感。
/// 一旦某主题字段升级成对象形态 → 走本函数 → TUI 自动跟随 `color_mode`。
fn resolve_multi_color(
    rgb: Option<&str>,
    ansi256: Option<u8>,
    ansi16: Option<&str>,
) -> Result<Color, String> {
    use crate::util::color_adapt::{ColorLevel, current};

    // 当前色阶精确匹配优先
    match current() {
        ColorLevel::Ansi16 => {
            if let Some(name) = ansi16 {
                return parse_color(name);
            }
        }
        ColorLevel::Ansi256 => {
            if let Some(idx) = ansi256 {
                return Ok(Color::Indexed(idx));
            }
        }
        _ => {}
    }

    // 回落顺序：rgb → ansi256 → ansi16
    if let Some(hex) = rgb {
        return parse_color(hex);
    }
    if let Some(idx) = ansi256 {
        return Ok(Color::Indexed(idx));
    }
    if let Some(name) = ansi16 {
        return parse_color(name);
    }

    Err("color object must specify at least one of rgb/ansi256/ansi16".into())
}

/// 解析颜色字符串
fn parse_color(s: &str) -> Result<Color, String> {
    // #rrggbb 十六进制
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| e.to_string())?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| e.to_string())?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| e.to_string())?;
            return Ok(Color::Rgb(r, g, b));
        }
        return Err(format!("invalid hex color: {s}"));
    }

    // ANSI 命名色
    Ok(match s {
        "reset" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" => Color::DarkGray,
        "light_red" => Color::LightRed,
        "light_green" => Color::LightGreen,
        "light_yellow" => Color::LightYellow,
        "light_blue" => Color::LightBlue,
        "light_magenta" => Color::LightMagenta,
        "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        _ => return Err(format!("unknown color name: {s}")),
    })
}

/// 从 JSON 字符串解析 Theme（供 impls 模块调用）
pub(super) fn parse_theme_json(json_str: &str, path: &str) -> Result<Theme, String> {
    let theme_json: ThemeJson =
        serde_json::from_str(json_str).map_err(|e| format!("parse theme {path}: {e}"))?;
    Ok(theme_json.into())
}
