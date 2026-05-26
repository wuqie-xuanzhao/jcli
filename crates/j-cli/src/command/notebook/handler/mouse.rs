//! 鼠标事件处理。
//!
//! 包含 TUI 界面中的鼠标点击、拖拽、滚轮等事件处理逻辑。

use crate::command::notebook::app::{AppMode, FlatEntryKind, Focus, NotebookApp};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

/// 鼠标事件处理时需要的布局信息
pub struct MouseLayoutInfo {
    /// 主区域
    pub main_area: Rect,
    /// 笔记列表区域（仅在 Normal 模式有效）
    pub list_area: Option<Rect>,
    /// 预览区域（仅在 Normal 模式有效）
    pub preview_area: Option<Rect>,
    /// 分割线 x 坐标（列表区和预览区的交界列）
    pub divider_x: Option<u16>,
}

/// 处理鼠标事件
#[allow(clippy::too_many_arguments)]
pub fn handle_mouse_event(
    app: &mut NotebookApp,
    mouse: MouseEvent,
    layout: &MouseLayoutInfo,
    editor_area: Rect,
) {
    if app.mode != AppMode::Normal {
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(app, mouse.column, mouse.row, layout, editor_area);
        }
        MouseEventKind::Drag(MouseButton::Left) => handle_drag(app, mouse.column, layout),
        MouseEventKind::Up(MouseButton::Left) => handle_mouse_up(app),
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            handle_scroll(
                app,
                mouse.column,
                mouse.row,
                layout,
                mouse.kind,
                editor_area,
            );
        }
        _ => {}
    }
}

/// 检查点是否在矩形区域内（含边界）
fn rect_contains(area: Rect, col: u16, row: u16) -> bool {
    col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
}

#[allow(clippy::too_many_arguments)]
/// 处理左键点击
fn handle_left_click(
    app: &mut NotebookApp,
    col: u16,
    row: u16,
    layout: &MouseLayoutInfo,
    editor_area: Rect,
) {
    // 检测是否点击分割线（优先级最高）
    if let Some(divider_x) = layout.divider_x
        && col >= divider_x.saturating_sub(2)
        && col <= divider_x + 2
        && row >= layout.main_area.y
        && row < layout.main_area.y + layout.main_area.height
    {
        app.is_dragging_panel = true;
        return;
    }

    // 点击编辑区：切换焦点到编辑器，并传递点击事件
    if rect_contains(editor_area, col, row) {
        app.focus = Focus::Editor;
        if let Some(ref mut editor) = app.editor {
            let mouse_event = MouseEvent {
                column: col,
                row,
                kind: MouseEventKind::Down(MouseButton::Left),
                modifiers: crossterm::event::KeyModifiers::empty(),
            };
            editor.handle_mouse(mouse_event, editor_area);
        }
        return;
    }

    // 点击列表区：选择笔记并切换焦点到列表
    if let Some(list_area) = layout.list_area
        && rect_contains(list_area, col, row)
    {
        app.focus = Focus::Tree;

        let inner_y = row.saturating_sub(list_area.y).saturating_sub(1);
        let max_visible = list_area.height.saturating_sub(2) as usize;

        if (inner_y as usize) < max_visible {
            let index = app.state.offset() + inner_y as usize;
            if index < app.flat_entries.len() {
                let now = std::time::Instant::now();

                let is_double_click = app
                    .last_click_time
                    .map(|t| now.duration_since(t).as_millis() < 500)
                    .unwrap_or(false)
                    && app.last_click_index == Some(index);

                app.state.select(Some(index));
                app.load_editor_for_selected();

                app.last_click_time = Some(now);
                app.last_click_pos = Some((col, row));
                app.last_click_index = Some(index);

                // 双击文件：切换焦点到编辑器
                if is_double_click {
                    let entry = &app.flat_entries[index];
                    if matches!(&entry.kind, FlatEntryKind::File { .. }) {
                        app.focus = Focus::Editor;
                    } else if let FlatEntryKind::Dir { dir_path, .. } = &entry.kind {
                        app.expanded_dirs.toggle(dir_path);
                        crate::command::notebook::app::io::save_expanded_dirs(&app.expanded_dirs);
                        app.build_flat_entries();
                        app.load_editor_for_selected();
                    }
                }
            }
        }
    }
}

/// 处理鼠标拖拽（调整面板比例）
fn handle_drag(app: &mut NotebookApp, col: u16, layout: &MouseLayoutInfo) {
    if !app.is_dragging_panel {
        return;
    }

    let frame_width = layout.main_area.width;
    if frame_width == 0 {
        return;
    }

    let relative_x = col.saturating_sub(layout.main_area.x);
    let new_ratio = (relative_x as u32 * 100 / frame_width as u32) as u16;
    app.panel_ratio = new_ratio.clamp(15, 60);
}

/// 处理鼠标释放
fn handle_mouse_up(app: &mut NotebookApp) {
    if app.is_dragging_panel {
        app.is_dragging_panel = false;
        crate::command::notebook::app::io::save_panel_ratio(app.panel_ratio);
    }
}

#[allow(clippy::too_many_arguments)]
/// 处理滚轮滚动
fn handle_scroll(
    app: &mut NotebookApp,
    col: u16,
    row: u16,
    layout: &MouseLayoutInfo,
    kind: MouseEventKind,
    editor_area: Rect,
) {
    // 编辑区滚轮：传递给编辑器
    if rect_contains(editor_area, col, row)
        && app.focus == Focus::Editor
        && let Some(ref mut editor) = app.editor
    {
        let mouse_event = MouseEvent {
            column: col,
            row,
            kind,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        editor.handle_mouse(mouse_event, editor_area);
        return;
    }

    // 列表区滚轮：切换选择项
    if let Some(list_area) = layout.list_area
        && rect_contains(list_area, col, row)
    {
        match kind {
            MouseEventKind::ScrollUp => app.move_up(),
            MouseEventKind::ScrollDown => app.move_down(),
            _ => {}
        }
    }
}

/// 计算鼠标事件处理所需的布局信息
pub fn compute_mouse_layout(frame_area: Rect, app: &NotebookApp) -> MouseLayoutInfo {
    // 主区域：标题栏之后、状态栏之前
    let main_area = Rect {
        x: frame_area.x,
        y: frame_area.y + 3,
        width: frame_area.width,
        height: frame_area.height.saturating_sub(7),
    };

    // Normal/CommandPopup 模式下计算列表/预览区域
    let (list_area, preview_area, divider_x) =
        if matches!(app.mode, AppMode::Normal | AppMode::CommandPopup) {
            let list_width = frame_area.width * app.panel_ratio / 100;
            let preview_width = frame_area.width.saturating_sub(list_width);
            (
                Some(Rect {
                    x: frame_area.x,
                    y: main_area.y,
                    width: list_width,
                    height: main_area.height,
                }),
                Some(Rect {
                    x: frame_area.x + list_width,
                    y: main_area.y,
                    width: preview_width,
                    height: main_area.height,
                }),
                Some(frame_area.x + list_width),
            )
        } else {
            (None, None, None)
        };

    MouseLayoutInfo {
        main_area,
        list_area,
        preview_area,
        divider_x,
    }
}
