use ratatui::style::Color;

// Re-export ThemeName from j-cli-core
pub use j_agent::theme_name::ThemeName;

/// 主题配色方案
/// 将所有 UI 颜色归类为语义化字段，方便统一管理
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct Theme {
    // ===== 全局背景 =====
    /// 主背景色
    pub bg_primary: Color,
    /// 标题栏背景
    pub bg_title: Color,
    /// 输入区背景
    pub bg_input: Color,
    /// 帮助/配置界面背景
    pub bg_panel: Color,

    // ===== 边框 =====
    /// 标题栏边框
    pub border_title: Color,
    /// 消息区边框
    pub border_message: Color,
    /// 输入区边框（正常）
    pub border_input: Color,
    /// 输入区边框（加载中）
    pub border_input_loading: Color,
    /// 配置界面边框
    pub border_config: Color,
    /// 分隔线
    pub separator: Color,

    // ===== 气泡 =====
    /// AI 气泡背景
    pub bubble_ai: Color,
    /// AI 气泡背景（选中时）
    pub bubble_ai_selected: Color,
    /// 用户气泡背景
    pub bubble_user: Color,
    /// 用户气泡背景（选中时）
    pub bubble_user_selected: Color,

    // ===== 标签 =====
    /// AI 标签颜色
    pub label_ai: Color,
    /// 用户标签颜色
    pub label_user: Color,
    /// 选中标签颜色
    pub label_selected: Color,

    // ===== 文字 =====
    /// 正文颜色
    pub text_normal: Color,
    /// 强调色（加粗文本）
    pub text_bold: Color,
    /// 弱化文字
    pub text_dim: Color,
    /// 非常弱化的文字
    pub text_very_dim: Color,
    /// 白色文字（用于输入区等）
    pub text_white: Color,
    /// 系统消息颜色
    pub text_system: Color,

    // ===== 标题栏元素 =====
    /// 标题栏图标色
    pub title_icon: Color,
    /// 标题栏分隔符
    pub title_separator: Color,
    /// 模型名称颜色
    pub title_model: Color,
    /// 消息计数颜色
    pub title_count: Color,
    /// 加载中文字颜色
    pub title_loading: Color,

    // ===== 输入区 =====
    /// 输入提示符颜色
    pub input_prompt: Color,
    /// 输入提示符（加载中）颜色
    pub input_prompt_loading: Color,
    /// 光标前景
    pub cursor_fg: Color,
    /// 光标背景
    pub cursor_bg: Color,

    // ===== 提示栏 =====
    /// 键位标签前景
    pub hint_key_fg: Color,
    /// 键位标签背景
    pub hint_key_bg: Color,
    /// 键位描述文字
    pub hint_desc: Color,
    /// 提示栏分隔符
    pub hint_separator: Color,

    // ===== Toast =====
    /// 成功 Toast 边框
    pub toast_success_border: Color,
    /// 成功 Toast 背景
    pub toast_success_bg: Color,
    /// 成功 Toast 文字
    pub toast_success_text: Color,
    /// 错误 Toast 边框
    pub toast_error_border: Color,
    /// 错误 Toast 背景
    pub toast_error_bg: Color,
    /// 错误 Toast 文字
    pub toast_error_text: Color,

    // ===== 工具确认区 =====
    /// 工具确认区边框
    pub tool_confirm_border: Color,
    /// 工具确认区背景
    pub tool_confirm_bg: Color,
    /// 工具确认区标题颜色
    pub tool_confirm_title: Color,
    /// 工具确认区工具名颜色
    pub tool_confirm_name: Color,
    /// 工具确认区消息文字颜色
    pub tool_confirm_text: Color,
    /// 工具确认区标签颜色（如"工具:"）
    pub tool_confirm_label: Color,
    /// 工具确认区提示文字颜色
    pub tool_confirm_hint: Color,

    // ===== 欢迎界面 =====
    /// 欢迎框边框
    pub welcome_border: Color,
    /// 欢迎文字
    pub welcome_text: Color,
    /// 欢迎提示文字
    pub welcome_hint: Color,
    /// 欢迎框诗句颜色
    pub welcome_quote: Color,
    /// 欢迎框渐变调色板索引（0-7），对应不同的渐变色组集合
    pub welcome_palette: u8,

    // ===== 模型选择 =====
    /// 模型选择框边框
    pub model_sel_border: Color,
    /// 模型选择框标题
    pub model_sel_title: Color,
    /// 活跃模型颜色
    pub model_sel_active: Color,
    /// 非活跃模型颜色
    pub model_sel_inactive: Color,
    /// 选中高亮背景
    pub model_sel_highlight_bg: Color,
    /// 选中高亮前景
    pub model_sel_highlight_fg: Color,

    // ===== 配置界面 =====
    /// 配置标题颜色
    pub config_title: Color,
    /// 配置分类标题颜色
    pub config_section: Color,
    /// 配置选中指针颜色
    pub config_pointer: Color,
    /// 配置选中标签颜色
    pub config_label_selected: Color,
    /// 配置普通标签颜色
    pub config_label: Color,
    /// 配置值颜色
    pub config_value: Color,
    /// 配置编辑背景
    pub config_edit_bg: Color,
    /// 配置 tab 选中背景
    pub config_tab_active_bg: Color,
    /// 配置 tab 选中前景
    pub config_tab_active_fg: Color,
    /// 配置 tab 非选中颜色
    pub config_tab_inactive: Color,
    /// 配置键位说明颜色
    pub config_hint_key: Color,
    /// 配置描述颜色
    pub config_hint_desc: Color,
    /// 配置 toggle 开启颜色
    pub config_toggle_on: Color,
    /// 配置 toggle 关闭颜色
    pub config_toggle_off: Color,
    /// 配置弱化文字
    pub config_dim: Color,
    /// API Key 隐藏颜色
    pub config_api_key: Color,

    // ===== Markdown 渲染 =====
    /// 标题 h1 颜色
    pub md_h1: Color,
    /// 标题 h2 颜色
    pub md_h2: Color,
    /// 标题 h3 颜色
    pub md_h3: Color,
    /// 标题 h4+ 颜色
    pub md_h4: Color,
    /// 标题分隔线
    pub md_heading_sep: Color,
    /// 行内代码前景
    pub md_inline_code_fg: Color,
    /// 行内代码背景
    pub md_inline_code_bg: Color,
    /// 列表符号颜色
    pub md_list_bullet: Color,
    /// 引用块竖线颜色
    pub md_blockquote_bar: Color,
    /// 引用块文字颜色
    pub md_blockquote_text: Color,
    /// 引用块背景颜色
    pub md_blockquote_bg: Color,
    /// 分隔线颜色
    pub md_rule: Color,
    /// 链接/URL 颜色
    pub md_link: Color,

    // ===== 代码块 =====
    /// 代码块边框颜色
    pub code_border: Color,
    /// 代码块背景
    pub code_bg: Color,
    /// 代码默认文字颜色
    pub code_default: Color,
    /// 关键字颜色
    pub code_keyword: Color,
    /// 字符串颜色
    pub code_string: Color,
    /// 注释颜色
    pub code_comment: Color,
    /// 数字颜色
    pub code_number: Color,
    /// 类型名颜色
    pub code_type: Color,
    /// 原始类型颜色
    pub code_primitive: Color,
    /// 宏调用颜色
    pub code_macro: Color,
    /// 属性/装饰器颜色
    pub code_attribute: Color,
    /// 生命周期颜色
    pub code_lifetime: Color,
    /// Shell 变量颜色
    pub code_shell_var: Color,

    // ===== 表格 =====
    /// 表格边框颜色
    pub table_border: Color,
    /// 表格表头颜色
    pub table_header: Color,
    /// 表格内容颜色
    pub table_body: Color,

    // ===== 帮助界面 =====
    /// 帮助标题颜色
    pub help_title: Color,
    /// 帮助按键颜色
    pub help_key: Color,
    /// 帮助描述颜色
    pub help_desc: Color,
    /// 帮助文件路径颜色
    pub help_path: Color,
    /// 帮助背景颜色
    pub help_bg: Color,

    // ===== Diff 显示 =====
    /// 新增行颜色（绿色）
    pub diff_add: Color,
    /// 删除行颜色（红色）
    pub diff_del: Color,
    /// diff header 颜色（青色）
    pub diff_header: Color,
}
