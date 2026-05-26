//! 自研 Markdown 编辑器
//!
//! 完全摆脱 tui-textarea 依赖，支持自动折行、Vim 模式等。

mod api;
mod commands;
mod input;
mod render;
mod selection;

pub use api::{
    MarkdownEditorOpts, open_markdown_editor, open_markdown_editor_on_terminal,
    open_markdown_editor_with_content,
};
pub use selection::RenderedVL;

use super::{
    history::Snapshot,
    renderer::MarkdownRenderer,
    search::SearchState,
    text_buffer::TextBuffer,
    theme::{EditorTheme, HighlightFn},
    vim::{Mode, Vim},
    wrap_engine::WrapEngine,
};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

/// 编辑器事件轮询间隔（约 60fps）。
const EDITOR_POLL_MS: u64 = 16;

/// 主题画廊项（显示名称 + 主题ID + 主题）
pub type ThemeGalleryItem = (&'static str, &'static str, EditorTheme);

/// 编辑器初始光标策略
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CursorPolicy {
    /// 光标在文件开头（默认，向后兼容）
    #[default]
    StartOfFile,
    /// 光标在文件末尾
    EndOfFile,
}

/// 视口/滚动状态
struct ViewportState {
    /// 垂直滚动偏移（视觉行级别）
    scroll_offset: usize,
    /// 视口高度
    height: usize,
    /// 视口宽度
    width: usize,
    /// 滚轮滚动锁定：防止 render() 自动将视口拉回到光标位置
    scroll_locked: bool,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            scroll_offset: 0,
            height: 20,
            width: 80,
            scroll_locked: false,
        }
    }
}

/// 主题管理状态
struct ThemeState {
    /// 主题画廊（名称 + 主题列表）
    gallery: Vec<ThemeGalleryItem>,
    /// 当前主题在画廊中的索引
    current_index: usize,
    /// 主题选择弹窗选中项索引
    popup_selected: usize,
    /// 用户在主题画廊中选择的主题ID（退出时返回）
    selected_id: Option<&'static str>,
}

/// 渲染行元数据映射
#[derive(Default)]
struct RenderMeta {
    /// 每个屏幕行对应一个 RenderedVL（每次渲染时更新，用于鼠标点击定位）
    vl_map: Vec<RenderedVL>,
    /// 当前屏幕顶部对应的渲染行索引（在 vl_map 中的偏移）
    map_index: usize,
    /// 上一次渲染时的实际渲染行总数（用于鼠标滚动计算 max_offset）
    /// 注意：wrap_engine 的 visual_line_count() 是源码行的视觉行计数，
    /// 但表格等元素渲染后会产生更多行。此字段存储实际渲染输出行数，
    /// 确保鼠标滚动能到达表格底部。
    rendered_line_count: usize,
}

/// 编辑器主结构
pub struct MarkdownEditor {
    // ---- 核心引擎 ----
    /// 文本缓冲区
    buffer: TextBuffer,
    /// 折行引擎
    wrap: WrapEngine,
    /// Vim 引擎
    vim: Vim,
    /// 搜索状态
    search: SearchState,
    /// 渲染器
    renderer: MarkdownRenderer,
    /// 主题
    theme: EditorTheme,

    // ---- 分组状态 ----
    /// 视口/滚动状态
    viewport: ViewportState,
    /// 主题管理状态
    themes: ThemeState,
    /// 渲染行元数据映射
    render_meta: RenderMeta,

    // ---- UI 杂项 ----
    /// 标题
    title: String,
    /// 命令面板选中项索引
    cmd_popup_selected: usize,
    /// 状态消息（短暂显示，下次按键清除）
    status_message: Option<String>,
    /// 进入搜索前的光标位置，用于 Esc 恢复
    cursor_before_search: Option<(usize, usize)>,
    /// Insert 模式命令面板的锚点（触发的 `/` 字符在 buffer 中的逻辑位置）
    /// 用于把 popup 渲染到 `/` 下方，而不是固定在编辑区底部
    insert_panel_anchor: Option<(usize, usize)>,
    /// 鼠标拖拽锚点（左键按下时的逻辑位置）
    mouse_anchor: Option<(usize, usize)>,
}

impl MarkdownEditor {
    /// 创建新的编辑器
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        title: &str,
        content: &str,
        theme: EditorTheme,
        highlight_fn: HighlightFn,
        theme_gallery: Vec<ThemeGalleryItem>,
        cursor_policy: CursorPolicy,
    ) -> Self {
        let mut buffer = TextBuffer::from_content(content);
        let initial_mode = if content.is_empty() {
            Mode::Insert
        } else {
            Mode::Normal
        };

        // 根据策略移动光标
        if cursor_policy == CursorPolicy::EndOfFile {
            buffer.move_cursor_bottom();
        }

        let mut vim = Vim::new(initial_mode.clone());
        vim.push_snapshot(Snapshot::new(buffer.snapshot()), buffer.cursor());

        let mut wrap = WrapEngine::new();
        wrap.rebuild_cache(buffer.lines());

        let renderer = MarkdownRenderer::new(theme.clone(), highlight_fn);

        let viewport_width: usize = 80; // 默认值，会在渲染时更新
        wrap.set_width(viewport_width.saturating_sub(6));

        // 查找当前主题在画廊中的索引
        let theme_index = theme_gallery
            .iter()
            .position(|(_, _, t)| *t == theme)
            .unwrap_or(0);

        Self {
            buffer,
            wrap,
            vim,
            search: SearchState::new(),
            renderer,
            theme,
            viewport: ViewportState::default(),
            themes: ThemeState {
                gallery: theme_gallery,
                current_index: theme_index,
                popup_selected: theme_index,
                selected_id: None,
            },
            render_meta: RenderMeta::default(),
            title: title.to_string(),
            cmd_popup_selected: 0,
            status_message: None,
            cursor_before_search: None,
            insert_panel_anchor: None,
            mouse_anchor: None,
        }
    }

    /// 获取用户选择的主题ID（退出时读取）
    pub fn selected_theme_id(&self) -> Option<&'static str> {
        self.themes.selected_id
    }

    /// 获取编辑器当前全部文本内容
    pub fn content(&self) -> String {
        self.buffer.lines().join("\n")
    }

    /// 判断编辑器是否处于"空闲 Normal 模式"（可安全拦截 Esc）
    ///
    /// 当 vim 处于 Normal 模式且无活跃搜索时返回 true。
    /// 此状态下 Esc 对编辑器无实际作用，外部可安全拦截用于焦点切换。
    pub fn is_idle_normal_mode(&self) -> bool {
        self.vim.mode() == &Mode::Normal && !self.search.is_searching()
    }

    /// 获取光标所在的视觉行
    pub fn cursor_visual_line(&self) -> usize {
        let (row, col) = self.buffer.cursor();
        self.wrap.logical_to_visual(row, col)
    }

    /// 视觉行上移（折行感知）
    pub fn move_cursor_visual_up(&mut self) {
        use crate::util::text::char_width;

        let current_visual = self.cursor_visual_line();
        if current_visual == 0 {
            return;
        }
        let target_visual = current_visual - 1;
        let (current_row, current_col) = self.buffer.cursor();

        // 确保目标行的缓存已构建
        let (target_logical, _) = self.wrap.visual_to_logical(target_visual);
        self.wrap
            .build_range(self.buffer.lines(), target_logical, target_logical + 1);

        if let Some(target_vl) = self.wrap.get_visual_line(target_visual) {
            let logical_line = target_vl.logical_line;
            let end_col = target_vl.end_col;
            let start_col = target_vl.start_col;

            // 保持视觉列位置：计算当前光标在当前视觉行中的屏幕偏移
            let current_vl = self.wrap.get_visual_line(current_visual);
            let current_start_col = current_vl.map(|vl| vl.start_col).unwrap_or(0);
            let current_line_text = self.buffer.line(current_row).map_or("", |v| v);
            let visual_x: usize = current_line_text
                .chars()
                .skip(current_start_col)
                .take(current_col.saturating_sub(current_start_col))
                .map(char_width)
                .sum();

            // 在目标视觉行中找到最接近该视觉 X 的逻辑列
            let target_line_text = self.buffer.line(logical_line).map_or("", |v| v);
            let new_col = if target_line_text.is_empty() {
                0
            } else {
                let segment: String = target_line_text.chars().skip(start_col).collect();
                Self::screen_col_to_char_offset(&segment, visual_x) + start_col
            };
            let new_col = new_col.min(end_col);
            self.buffer.set_cursor(logical_line, new_col);
        }
    }

    /// 视觉行下移（折行感知）
    pub fn move_cursor_visual_down(&mut self) {
        use crate::util::text::char_width;

        let current_visual = self.cursor_visual_line();
        let total_visual = self.wrap.visual_line_count();
        if current_visual >= total_visual.saturating_sub(1) {
            return;
        }
        let target_visual = current_visual + 1;
        let (current_row, current_col) = self.buffer.cursor();

        // 确保目标行的缓存已构建
        let (target_logical, _) = self.wrap.visual_to_logical(target_visual);
        self.wrap
            .build_range(self.buffer.lines(), target_logical, target_logical + 1);

        if let Some(target_vl) = self.wrap.get_visual_line(target_visual) {
            let logical_line = target_vl.logical_line;
            let end_col = target_vl.end_col;
            let start_col = target_vl.start_col;

            // 保持视觉列位置：计算当前光标在当前视觉行中的屏幕偏移
            let current_vl = self.wrap.get_visual_line(current_visual);
            let current_start_col = current_vl.map(|vl| vl.start_col).unwrap_or(0);
            let current_line_text = self.buffer.line(current_row).map_or("", |v| v);
            let visual_x: usize = current_line_text
                .chars()
                .skip(current_start_col)
                .take(current_col.saturating_sub(current_start_col))
                .map(char_width)
                .sum();

            // 在目标视觉行中找到最接近该视觉 X 的逻辑列
            let target_line_text = self.buffer.line(logical_line).map_or("", |v| v);
            let new_col = if target_line_text.is_empty() {
                0
            } else {
                let segment: String = target_line_text.chars().skip(start_col).collect();
                Self::screen_col_to_char_offset(&segment, visual_x) + start_col
            };
            let new_col = new_col.min(end_col);
            self.buffer.set_cursor(logical_line, new_col);
        }
    }

    /// 撤销
    pub fn undo(&mut self) {
        if let Some(snap) = self.vim.undo() {
            self.buffer.replace_lines(snap.lines.clone());
            self.buffer.set_cursor(snap.cursor.0, snap.cursor.1);
            self.rebuild_wrap_cache();
        }
    }

    /// 重做
    pub fn redo(&mut self) {
        if let Some(snap) = self.vim.redo() {
            self.buffer.replace_lines(snap.lines.clone());
            self.buffer.set_cursor(snap.cursor.0, snap.cursor.1);
            self.rebuild_wrap_cache();
        }
    }

    /// 搜索下一个匹配
    pub fn search_next(&mut self) {
        self.search.next_match();
        if let Some(m) = self.search.current_match() {
            self.buffer.set_cursor(m.line, m.start);
        }
    }

    /// 搜索上一个匹配
    pub fn search_prev(&mut self) {
        self.search.prev_match();
        if let Some(m) = self.search.current_match() {
            self.buffer.set_cursor(m.line, m.start);
        }
    }

    /// 重建折行缓存
    fn rebuild_wrap_cache(&mut self) {
        // 先确保代码块缓存有效，获取代码块范围
        self.renderer.ensure_cache_valid(self.buffer.lines());
        let cb_ranges = self.renderer.code_block_content_ranges();
        self.wrap
            .rebuild_cache_with_code_blocks(self.buffer.lines(), &cb_ranges);
        // 同时使渲染器缓存失效（语法高亮等）
        self.renderer.invalidate_cache();
    }

    /// 更新滚动偏移（基于视觉位置）
    fn update_scroll_from_visual(&mut self, visual_pos: usize, viewport_height: usize) {
        if visual_pos < self.viewport.scroll_offset {
            self.viewport.scroll_offset = visual_pos;
        } else if visual_pos >= self.viewport.scroll_offset + viewport_height {
            self.viewport.scroll_offset = visual_pos - viewport_height + 1;
        }
    }

    // ========== 鼠标操作 ==========

    /// 将屏幕坐标转换为逻辑位置 (logical_row, logical_col)。
    ///
    /// 返回 `None` 表示点击在内容区域之外（边框、状态栏等）。
    #[allow(clippy::too_many_arguments)]
    fn screen_to_logical(
        &self,
        screen_x: u16,
        screen_y: u16,
        area: Rect,
    ) -> Option<(usize, usize)> {
        // 减去边框偏移，得到内容区域内的坐标
        let content_x = screen_x.saturating_sub(area.x + 1) as usize; // 左边框 1 列
        let content_y = screen_y.saturating_sub(area.y + 1) as usize; // 上边框 1 行

        let content_height = area.height.saturating_sub(3) as usize; // 上边框 + 下边框 + 状态栏
        let line_num_width = if self.renderer.is_show_line_numbers() {
            6
        } else {
            0
        };

        // 超出内容区域
        if content_y >= content_height {
            return None;
        }

        // 使用渲染行元数据映射，将屏幕行号转换为渲染行索引
        let rendered_row = content_y + self.render_meta.map_index;

        let vl_meta = self.render_meta.vl_map.get(rendered_row)?;

        let logical_row = vl_meta.logical_line;
        let vl_start_col = vl_meta.start_col;

        // 减去行号区域得到内容列
        let content_col = content_x.saturating_sub(line_num_width);

        // 获取该逻辑行的原始文本
        let line_text = self.buffer.line(logical_row)?;

        // 获取该视觉行实际渲染的文本段（从 start_col 开始的子串）
        let vl_text: String = line_text.chars().skip(vl_start_col).collect();

        // 将屏幕列转换为字符偏移（考虑宽字符）
        let logical_col = Self::screen_col_to_char_offset(&vl_text, content_col) + vl_start_col;

        // 限制到行尾
        let max_col = line_text.chars().count();
        let logical_col = logical_col.min(max_col);

        Some((logical_row, logical_col))
    }

    /// 将屏幕列号转换为字符偏移（考虑 CJK 等宽字符）。
    fn screen_col_to_char_offset(text: &str, screen_col: usize) -> usize {
        use crate::util::text::char_width;

        let mut acc_width = 0;
        for (i, ch) in text.chars().enumerate() {
            if acc_width >= screen_col {
                return i;
            }
            acc_width += char_width(ch);
        }
        text.chars().count()
    }

    /// 处理鼠标事件。
    pub fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some((row, col)) = self.screen_to_logical(mouse.column, mouse.row, area) {
                    // 点击有效区域：移动光标
                    self.vim.set_mode(Mode::Normal);
                    self.buffer.set_cursor(row, col);
                    self.mouse_anchor = Some((row, col));
                } else {
                    // 点击空白区域（边框、状态栏等）：取消选区
                    self.vim.set_mode(Mode::Normal);
                    self.mouse_anchor = None;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some((row, col)) = self.screen_to_logical(mouse.column, mouse.row, area) {
                    if let Some(anchor) = self.mouse_anchor
                        && *self.vim.mode() != Mode::Visual
                    {
                        // 进入 Visual 模式，选区起点为按下位置
                        self.vim.set_mode(Mode::Visual);
                        self.vim.set_visual_start(anchor);
                    }
                    self.buffer.set_cursor(row, col);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.mouse_anchor = None;
            }
            MouseEventKind::ScrollUp => {
                let step = 3;
                self.viewport.scroll_offset = self.viewport.scroll_offset.saturating_sub(step);
                self.viewport.scroll_locked = true;
            }
            MouseEventKind::ScrollDown => {
                let step = 3;
                let content_height = area.height.saturating_sub(3) as usize;
                let total_visual = self.wrap.visual_line_count();
                let max_offset = total_visual.saturating_sub(content_height);
                self.viewport.scroll_offset = (self.viewport.scroll_offset + step).min(max_offset);
                self.viewport.scroll_locked = true;
            }
            _ => {}
        }
    }

    /// 复制文本到系统剪贴板
    fn copy_to_clipboard(&self, text: &str) -> Result<(), String> {
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new().map_err(|e| format!("无法访问剪贴板: {e}"))?;
        clipboard
            .set_text(text)
            .map_err(|e| format!("复制到剪贴板失败: {e}"))?;
        Ok(())
    }
}

/// 编辑器动作
#[derive(Debug)]
pub enum EditorAction {
    /// 继续编辑
    Continue,
    /// 提交内容（保存并退出）
    Submit(String),
    /// 保存内容但不退出
    Save(String),
    /// 取消编辑
    Cancel,
}
