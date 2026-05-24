use crate::command::chat::app::ChatApp;
use crate::command::chat::app::{Action, ChatMode, ConfigTab, CursorDirection};
use crate::command::chat::app::{ContextMenu, MouseSelection};
use crate::command::chat::render::cache::copy_to_clipboard;
use crate::command::chat::ui::chat::{
    copy_selection_to_clipboard, extract_selection_text, screen_to_text_pos,
};
use crate::command::chat::ui::context_menu::is_point_in_menu;
use crate::command::chat::ui::help::{help_extract_selection_text, help_screen_to_text_pos};
use crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::execute;
use ratatui::layout::Rect;
use std::io;

// Re-import mode handlers from the handler module
use crate::command::chat::handler::{
    handle_agent_perm_confirm_mode, handle_archive_confirm_mode, handle_archive_list_mode,
    handle_browse_mode, handle_chat_mode, handle_config_mode, handle_plan_approval_confirm_mode,
    handle_select_model, handle_select_theme, handle_tool_confirm_mode,
};

// ── Mouse scroll routing ────────────────────────────────────────────────

/// 根据当前 ChatMode (及 ConfigTab) 将鼠标滚轮事件路由到对应的导航 Action。
pub(super) fn mouse_scroll_action(app: &ChatApp, dir: CursorDirection) -> Action {
    match app.ui.mode {
        ChatMode::Config if !app.ui.config_editing => match app.ui.config_tab {
            ConfigTab::Session => Action::SessionListNavigate(dir),
            ConfigTab::Archive => Action::ArchiveListNavigate(dir),
            ConfigTab::Teammates => Action::TeammatesNavigate(dir),
            ConfigTab::Tools | ConfigTab::Skills | ConfigTab::Hooks | ConfigTab::Commands => {
                Action::ToggleMenuNavigate(dir)
            }
            _ => Action::ConfigNavigate(dir),
        },
        ChatMode::SelectModel => Action::ModelSelectNavigate(dir),
        ChatMode::SelectTheme => Action::ThemeSelectNavigate(dir),
        ChatMode::ArchiveList => Action::ArchiveListNavigate(dir),
        _ => Action::Scroll(dir),
    }
}

// ── Config mouse helpers ────────────────────────────────────────────────

/// 处理配置界面的鼠标左键点击事件。
///
/// 返回 `Some(action)` 表示需要执行的 Action，返回 `None` 表示点击未命中任何可交互区域。
pub(super) fn config_mouse_click(app: &mut ChatApp, col: u16, row: u16) -> Option<Action> {
    if app.ui.config_editing {
        return None;
    }

    // ── 1. 检测 Tab 栏点击 ──
    if let Some(tab_bar_y) = app.ui.config_tab_bar_y
        && row == tab_bar_y
    {
        for hitbox in &app.ui.config_tab_hitboxes {
            if col >= hitbox.start_col && col < hitbox.end_col {
                if app.ui.config_tab != hitbox.tab {
                    return Some(Action::ConfigSwitchTabTo(hitbox.tab));
                }
                return None;
            }
        }
        return None;
    }

    // ── 2. 检测 Model tab 左侧 Provider 列表点击 ──
    if app.ui.config_tab == ConfigTab::Model
        && let Some(provider_area) = app.ui.config_provider_area
        && is_point_in_rect(col, row, provider_area)
    {
        let provider_lines = &app.ui.config_provider_lines;
        if provider_lines.is_empty() {
            return None;
        }
        let inner_y = (row - provider_area.y) as usize;
        let clicked_idx = match provider_lines.binary_search(&inner_y) {
            Ok(idx) => idx,
            Err(0) => return None,
            Err(idx) => idx - 1,
        };
        let current = app.ui.config_provider_idx;
        if clicked_idx == current && !app.ui.model_in_fields {
            return Some(Action::ModelToggleLevel);
        }
        return Some(Action::ConfigProviderSelect(clicked_idx));
    }

    // ── 3. 检测列表项点击 ──
    let list_area = app.ui.config_list_area?;
    if !is_point_in_rect(col, row, list_area) {
        return None;
    }

    let inner_y = (row - list_area.y) as usize;
    let content_y = inner_y + app.ui.config_scroll_offset as usize;

    let field_lines = &app.ui.config_field_lines;
    if field_lines.is_empty() {
        return None;
    }

    let clicked_idx = match field_lines.binary_search(&content_y) {
        Ok(idx) => idx,
        Err(0) => return None,
        Err(idx) => {
            let candidate = idx - 1;
            let candidate_start = field_lines[candidate];
            if content_y < candidate_start {
                return None;
            }
            candidate
        }
    };

    let current_idx = match app.ui.config_tab {
        ConfigTab::Session => app.ui.session_list_index,
        ConfigTab::Archive => app.ui.archive_list_index,
        ConfigTab::Teammates => app.ui.teammate_list_index,
        ConfigTab::Global if app.ui.compact_exempt_sublist => app.ui.compact_exempt_idx,
        ConfigTab::Model
        | ConfigTab::Tools
        | ConfigTab::Skills
        | ConfigTab::Hooks
        | ConfigTab::Commands
        | ConfigTab::Global => app.ui.config_field_idx,
    };

    if clicked_idx == current_idx {
        return Some(config_enter_action(app));
    }

    Some(config_select_action(app, clicked_idx))
}

/// 根据当前 ConfigTab 返回"选中指定索引"的 Action。
pub(super) fn config_select_action(app: &ChatApp, idx: usize) -> Action {
    match app.ui.config_tab {
        ConfigTab::Session => Action::SessionListSelect(idx),
        ConfigTab::Archive => Action::ArchiveListSelect(idx),
        ConfigTab::Teammates => Action::TeammatesSelect(idx),
        ConfigTab::Global if app.ui.compact_exempt_sublist => Action::CompactExemptSelect(idx),
        _ => Action::ConfigFieldSelect(idx),
    }
}

/// 根据当前 ConfigTab 返回"确认/进入"的 Action。
pub(super) fn config_enter_action(app: &ChatApp) -> Action {
    match app.ui.config_tab {
        ConfigTab::Session | ConfigTab::Archive | ConfigTab::Teammates => Action::ConfigEnter,
        ConfigTab::Tools | ConfigTab::Skills | ConfigTab::Hooks | ConfigTab::Commands => {
            Action::ToggleMenuToggle
        }
        _ => Action::ConfigEnter,
    }
}

/// 判断点 (col, row) 是否在 Rect 内部（不含 border）。
pub(super) fn is_point_in_rect(col: u16, row: u16, rect: Rect) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

// ── Context menu copy ───────────────────────────────────────────────────

/// 执行右键菜单的复制操作。
///
/// 优先级：有选区时复制选区内容，无选区时复制整条消息。
pub(super) fn execute_context_menu_copy(app: &mut ChatApp) {
    // 优先复制选区
    if app.ui.mouse_selection.is_some() {
        copy_selection_to_clipboard(app);
        app.ui.mouse_selection = None;
        return;
    }

    // 无选区：通过 global_line 定位消息并复制整条消息内容
    let global_line = match &app.ui.context_menu {
        Some(m) => m.global_line,
        None => return,
    };

    let cached = match app.ui.msg_lines_cache.as_ref() {
        Some(c) => c,
        None => return,
    };

    // 二分查找 global_line 所属消息索引
    let msg_index = {
        let mut lo = 0usize;
        let mut hi = cached.msg_start_lines.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let (_, start) = cached.msg_start_lines[mid];
            if start <= global_line {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo.saturating_sub(1)
    };

    // 从 display_messages 获取消息内容
    let content = {
        let display = crate::util::safe_lock(&app.display_messages, "ContextMenuCopy");
        display.get(msg_index).map(|msg| msg.content.clone())
    };

    if let Some(content) = content
        && !content.is_empty()
    {
        use crate::command::chat::render::cache::copy_to_clipboard;
        if copy_to_clipboard(&content) {
            app.show_toast("已复制到剪贴板", false);
        } else {
            app.show_toast("复制到剪贴板失败", true);
        }
    }
}

// ── Main event dispatch ─────────────────────────────────────────────────

/// 将单个 crossterm Event 分发到对应的 handler / Action。
/// 返回 true 表示应退出主循环。
pub(super) fn dispatch_event(
    app: &mut ChatApp,
    evt: Event,
    needs_redraw: &mut bool,
    mouse_capture_enabled: &mut bool,
) -> bool {
    match evt {
        Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
            // Ctrl+M: 切换鼠标捕获（滚动模式/选择模式）
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('m') {
                *mouse_capture_enabled = !*mouse_capture_enabled;
                if *mouse_capture_enabled {
                    let _ = execute!(io::stdout(), event::EnableMouseCapture);
                    app.show_toast("鼠标: 滚轮滚动 (Shift+拖拽可选中)", false);
                } else {
                    let _ = execute!(io::stdout(), event::DisableMouseCapture);
                    app.show_toast("鼠标: 自由选中 (Ctrl+M 切回滚轮)", false);
                }
                *needs_redraw = true;
                return false;
            }
            *needs_redraw = true;

            // 右键菜单快捷键拦截（最高优先级）
            if app.ui.context_menu.is_some() {
                match key.code {
                    KeyCode::Enter => {
                        execute_context_menu_copy(app);
                        app.ui.context_menu = None;
                        return false;
                    }
                    KeyCode::Esc => {
                        app.ui.context_menu = None;
                        return false;
                    }
                    _ => {}
                }
            }

            // 选区快捷键：c 复制、Esc 取消（优先级高于模式分发）
            if app.ui.mouse_selection.is_some() {
                match key.code {
                    KeyCode::Char('c') => {
                        let text = if matches!(app.ui.mode, ChatMode::Help) {
                            app.ui
                                .help_lines_cache
                                .as_ref()
                                .zip(app.ui.mouse_selection.as_ref())
                                .map(|(cached, sel)| {
                                    help_extract_selection_text(cached, sel.anchor, sel.current)
                                })
                                .unwrap_or_default()
                        } else if matches!(app.ui.mode, ChatMode::Config) {
                            app.ui
                                .config_lines_cache
                                .as_ref()
                                .zip(app.ui.mouse_selection.as_ref())
                                .map(|(cached, sel)| {
                                    help_extract_selection_text(cached, sel.anchor, sel.current)
                                })
                                .unwrap_or_default()
                        } else {
                            app.ui
                                .msg_lines_cache
                                .as_ref()
                                .zip(app.ui.mouse_selection.as_ref())
                                .map(|(cached, sel)| {
                                    extract_selection_text(cached, sel.anchor, sel.current)
                                })
                                .unwrap_or_default()
                        };
                        if !text.is_empty() {
                            if copy_to_clipboard(&text) {
                                app.show_toast("已复制到剪贴板", false);
                            } else {
                                app.show_toast("复制到剪贴板失败", true);
                            }
                        }
                        app.ui.mouse_selection = None;
                        return false;
                    }
                    KeyCode::Esc => {
                        app.ui.mouse_selection = None;
                        return false;
                    }
                    _ => {}
                }
            }

            match app.ui.mode {
                ChatMode::Chat => {
                    if handle_chat_mode(app, key) {
                        return true; // quit
                    }
                }
                ChatMode::SelectModel => handle_select_model(app, key),
                ChatMode::SelectTheme => handle_select_theme(app, key),
                ChatMode::Browse => handle_browse_mode(app, key),
                ChatMode::Help => match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.ui.help_scroll_offset = app.ui.help_scroll_offset.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.ui.help_scroll_offset = app.ui.help_scroll_offset.saturating_add(1);
                    }
                    KeyCode::PageUp => {
                        app.ui.help_scroll_offset = app.ui.help_scroll_offset.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        app.ui.help_scroll_offset = app.ui.help_scroll_offset.saturating_add(10);
                    }
                    _ => {
                        app.ui.mouse_selection = None;
                        app.update(Action::ExitToChat);
                    }
                },
                ChatMode::Config => handle_config_mode(app, key),
                ChatMode::ArchiveConfirm => handle_archive_confirm_mode(app, key),
                ChatMode::ArchiveList => handle_archive_list_mode(app, key),
                ChatMode::ToolConfirm => handle_tool_confirm_mode(app, key),
                ChatMode::AgentPermConfirm => handle_agent_perm_confirm_mode(app, key),
                ChatMode::PlanApprovalConfirm => handle_plan_approval_confirm_mode(app, key),
            }
            false
        }
        Event::Paste(text) => {
            if matches!(app.ui.mode, ChatMode::Chat) {
                for c in text.chars() {
                    if c == '\r' {
                        continue;
                    }
                    if c == '\n' {
                        app.ui.input_buffer.insert_newline();
                    } else {
                        app.ui.input_buffer.insert_char(c);
                    }
                }
                *needs_redraw = true;
            } else if matches!(app.ui.mode, ChatMode::Config) && app.ui.config_editing {
                for c in text.chars() {
                    if c == '\n' || c == '\r' {
                        continue;
                    }
                    app.update(Action::ConfigEditChar(c));
                }
                *needs_redraw = true;
            } else if matches!(app.ui.mode, ChatMode::ToolConfirm) {
                if app.ui.tool_ask_mode
                    && !app.ui.tool_interact_typing
                    && let Some(cur_q) = app.ui.tool_ask_questions.get(app.ui.tool_ask_current_idx)
                {
                    app.ui.tool_ask_cursor = cur_q.options.len();
                    app.ui.tool_interact_typing = true;
                    app.ui.tool_interact_input.clear();
                    app.ui.tool_interact_cursor = 0;
                }
                if app.ui.tool_interact_typing {
                    for c in text.chars() {
                        if c == '\r' {
                            continue;
                        }
                        if c == '\n' && !app.ui.tool_ask_mode {
                            continue;
                        }
                        if app.ui.tool_ask_mode {
                            app.update(Action::AskInputChar(c));
                        } else {
                            app.update(Action::ToolInteractInputChar(c));
                        }
                    }
                    *needs_redraw = true;
                }
            }
            false
        }
        Event::Resize(_, _) => {
            *needs_redraw = true;
            false
        }
        Event::Mouse(mouse) if *mouse_capture_enabled => match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.update(mouse_scroll_action(app, CursorDirection::Up));
                *needs_redraw = true;
                false
            }
            MouseEventKind::ScrollDown => {
                app.update(mouse_scroll_action(app, CursorDirection::Down));
                *needs_redraw = true;
                false
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if app.ui.context_menu.is_some() {
                    if is_point_in_menu(app, mouse.column, mouse.row) {
                        execute_context_menu_copy(app);
                    }
                    app.ui.context_menu = None;
                    *needs_redraw = true;
                    return false;
                }

                if matches!(app.ui.mode, ChatMode::Config) {
                    // 先检查是否在内容区域内（用于选区）
                    if let Some(inner) = app.ui.config_content_inner
                        && let Some(ref cached) = app.ui.config_lines_cache
                        && let Some((gline, coff)) = help_screen_to_text_pos(
                            mouse.column,
                            mouse.row,
                            inner,
                            app.ui.config_content_scroll as usize,
                            cached,
                        )
                    {
                        app.ui.mouse_selection = Some(MouseSelection {
                            anchor: (gline, coff),
                            current: (gline, coff),
                        });
                        *needs_redraw = true;
                    } else if let Some(action) = config_mouse_click(app, mouse.column, mouse.row) {
                        // 不在内容区域：走原来的 Tab 切换 / 列表选中逻辑
                        app.update(action);
                        *needs_redraw = true;
                    }
                    return false;
                }

                if matches!(app.ui.mode, ChatMode::Help) {
                    if let Some(inner) = app.ui.help_area_inner
                        && let Some(ref cached) = app.ui.help_lines_cache
                        && let Some((gline, coff)) = help_screen_to_text_pos(
                            mouse.column,
                            mouse.row,
                            inner,
                            app.ui.help_scroll_offset,
                            cached,
                        )
                    {
                        app.ui.mouse_selection = Some(MouseSelection {
                            anchor: (gline, coff),
                            current: (gline, coff),
                        });
                        *needs_redraw = true;
                    }
                    return false;
                }

                if let Some(inner) = app.ui.msg_area_inner
                    && let Some(ref cached) = app.ui.msg_lines_cache
                    && let Some((gline, coff)) = screen_to_text_pos(
                        mouse.column,
                        mouse.row,
                        inner,
                        app.ui.scroll_offset,
                        cached,
                    )
                {
                    app.ui.mouse_selection = Some(MouseSelection {
                        anchor: (gline, coff),
                        current: (gline, coff),
                    });
                    *needs_redraw = true;
                } else {
                    if app.ui.mouse_selection.is_some() {
                        app.ui.mouse_selection = None;
                        *needs_redraw = true;
                    }
                }
                false
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if let Some(inner) = app.ui.msg_area_inner
                    && let Some(ref cached) = app.ui.msg_lines_cache
                    && let Some((gline, _)) = screen_to_text_pos(
                        mouse.column,
                        mouse.row,
                        inner,
                        app.ui.scroll_offset,
                        cached,
                    )
                {
                    app.ui.context_menu = Some(ContextMenu {
                        global_line: gline,
                        screen_pos: (mouse.column, mouse.row),
                    });
                    *needs_redraw = true;
                } else {
                    if app.ui.context_menu.is_some() {
                        app.ui.context_menu = None;
                        *needs_redraw = true;
                    }
                }
                false
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Config 模式：拖拽更新选区
                if matches!(app.ui.mode, ChatMode::Config) {
                    if let Some(inner) = app.ui.config_content_inner
                        && let Some(ref cached) = app.ui.config_lines_cache
                        && let Some((gline, coff)) = help_screen_to_text_pos(
                            mouse.column,
                            mouse.row,
                            inner,
                            app.ui.config_content_scroll as usize,
                            cached,
                        )
                        && let Some(ref mut sel) = app.ui.mouse_selection
                    {
                        sel.current = (gline, coff);
                        *needs_redraw = true;
                    }
                    return false;
                }
                if matches!(app.ui.mode, ChatMode::Help) {
                    if let Some(inner) = app.ui.help_area_inner
                        && let Some(ref cached) = app.ui.help_lines_cache
                        && let Some((gline, coff)) = help_screen_to_text_pos(
                            mouse.column,
                            mouse.row,
                            inner,
                            app.ui.help_scroll_offset,
                            cached,
                        )
                        && let Some(ref mut sel) = app.ui.mouse_selection
                    {
                        sel.current = (gline, coff);
                        *needs_redraw = true;
                    }
                    return false;
                }
                if let Some(inner) = app.ui.msg_area_inner
                    && let Some(ref cached) = app.ui.msg_lines_cache
                    && let Some((gline, coff)) = screen_to_text_pos(
                        mouse.column,
                        mouse.row,
                        inner,
                        app.ui.scroll_offset,
                        cached,
                    )
                    && let Some(ref mut sel) = app.ui.mouse_selection
                {
                    sel.current = (gline, coff);
                    *needs_redraw = true;
                }
                false
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(ref sel) = app.ui.mouse_selection
                    && sel.anchor == sel.current
                {
                    app.ui.mouse_selection = None;
                }
                false
            }
            _ => false,
        },
        _ => false,
    }
}
