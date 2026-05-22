//! 消息渲染缓存模块
//!
//! 负责增量构建所有消息的渲染行，按职责拆分为以下子模块：
//! - `bubble` — 气泡布局工具
//! - `msg_render` — 用户/AI 消息渲染
//! - `confirm_render` — 确认/交互区域渲染
//! - `tool_call_render` — 工具调用请求渲染
//! - `tool_result_render` — 工具结果渲染
//! - `animation` — 动画效果
//! - `clipboard` — 剪贴板操作
//! - `incremental_build` — 增量构建（全量/流式重建）

pub mod animation;
pub mod bubble;
pub mod clipboard;
pub mod confirm_render;
pub mod incremental_build;
pub mod msg_render;
pub mod tool_call_render;
pub mod tool_result_render;

// ── Re-export 公共 API（保持外部引用路径不变）──
pub use clipboard::copy_to_clipboard;
pub use incremental_build::{build_message_lines_incremental, rebuild_streaming_only};

use super::theme::Theme;
use ratatui::text::Line;

// ── 模块级常量（提取自各渲染函数中的魔法值）──

/// `render_thinking_block` 折叠模式下最大显示行数
pub(crate) const THINKING_FOLDED_MAX_LINES: usize = 5;
/// `render_assistant_msg` 气泡最小宽度（字符列数）
pub(crate) const BUBBLE_MIN_WIDTH: usize = 20;
/// `render_assistant_msg` 气泡左边距（字符列数），避免气泡贴近消息区左边界
pub(crate) const ASSISTANT_BUBBLE_LEFT_MARGIN: usize = 2;
/// `render_user_msg` 用户气泡左右内边距（字符列数）
pub(crate) const USER_BUBBLE_PAD_LR: usize = 3;
/// `render_tool_result_msg` / `render_bash_result` 普通结果截断显示的行数上限
pub(crate) const TOOL_RESULT_DISPLAY_MAX_LINES: usize = 100;
/// Plan 内容折叠时最大显示行数。
pub(crate) const PLAN_DISPLAY_MAX_LINES: usize = 20;

// ── 渲染上下文结构体（提取自多参数渲染函数的公共参数）──

/// 消息渲染的公共上下文
pub struct RenderContext<'a> {
    pub bubble_max_width: usize,
    pub lines: &'a mut Vec<Line<'static>>,
    pub theme: &'a Theme,
    pub expand: bool,
    /// 气泡背景色与主背景色一致（扁平效果）
    pub flat_bubble: bool,
}

/// 内容渲染的公共上下文（用于工具结果等）
pub(crate) struct ContentContext<'a> {
    pub content_w: usize,
    pub lines: &'a mut Vec<Line<'static>>,
    pub theme: &'a Theme,
    pub expand: bool,
}

/// 在 Markdown 内容中查找一个安全的截断边界，确保不会在代码围栏中间截断。
pub fn find_stable_boundary(content: &str) -> usize {
    // 统计 ``` 出现次数，奇数说明有未闭合的代码块
    let mut fence_count = 0usize;
    let mut last_safe_boundary = 0usize;
    let mut i = 0;
    let bytes = content.as_bytes();
    while i < bytes.len() {
        // 检测 ``` 围栏
        if i + 2 < bytes.len() && bytes[i] == b'`' && bytes[i + 1] == b'`' && bytes[i + 2] == b'`' {
            fence_count += 1;
            i += 3;
            // 跳过同行剩余内容（语言标识等）
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // 检测 \n\n 段落边界
        if i + 1 < bytes.len() && bytes[i] == b'\n' && bytes[i + 1] == b'\n' {
            // 只有在代码块外才算安全边界
            if fence_count.is_multiple_of(2) {
                last_safe_boundary = i + 2; // 指向下一段的起始位置
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    last_safe_boundary
}
