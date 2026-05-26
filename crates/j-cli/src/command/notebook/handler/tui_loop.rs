//! TUI 主循环。
//!
//! 包含 Notebook TUI 应用的主事件循环和初始化逻辑。

use crate::command::notebook::app::{
    AppMode, FlatEntryKind, Focus, NotebookApp, handle_command_popup_mode, handle_confirm_delete,
    handle_input_mode, handle_ratio_input_mode,
};
use crate::command::notebook::ui::draw_ui;
use crate::error;
use crossterm::event::{Event, KeyCode};
use crossterm::{
    event, execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

use super::mouse::{compute_mouse_layout, handle_mouse_event};

/// Notebook 事件轮询间隔（约 60fps）。
const NOTEBOOK_POLL_MS: u64 = 16;

/// 运行 Notebook TUI（入口函数，包含 panic 处理）
pub fn run_notebook_tui() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = execute!(io::stdout(), crossterm::event::DisableMouseCapture);
        default_hook(info);
    }));

    let result = run_notebook_tui_internal();

    let _ = std::panic::take_hook();

    if let Err(e) = result {
        error!("TUI 启动失败: {}", e);
    }
}

/// TUI 内部主循环
fn run_notebook_tui_internal() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, crossterm::event::EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = NotebookApp::new();

    loop {
        terminal.draw(|f| draw_ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(NOTEBOOK_POLL_MS))? {
            match event::read()? {
                Event::Key(key) => {
                    match app.mode {
                        AppMode::Normal => {
                            match app.focus {
                                Focus::Tree => {
                                    // 内联 Normal 模式 Tree 焦点按键处理
                                    match key.code {
                                        KeyCode::Esc => {
                                            if app.editor_dirty {
                                                app.save_editor_content();
                                            }
                                            app.should_exit = true;
                                        }
                                        KeyCode::Up | KeyCode::Char('k') => {
                                            app.move_up();
                                        }
                                        KeyCode::Down | KeyCode::Char('j') => {
                                            app.move_down();
                                        }
                                        KeyCode::Enter => {
                                            if let Some(entry) = app.selected_entry().cloned() {
                                                match &entry.kind {
                                                    FlatEntryKind::Dir { dir_path, .. } => {
                                                        app.expanded_dirs.toggle(dir_path);
                                                        super::super::app::io::save_expanded_dirs(&app.expanded_dirs);
                                                        app.build_flat_entries();
                                                    }
                                                    FlatEntryKind::File { .. } => {
                                                        app.focus = Focus::Editor;
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                    if app.should_exit {
                                        break;
                                    }
                                }
                                Focus::Editor => {
                                    if let Some(ref mut editor) = app.editor {
                                        // Esc 在 Normal 模式下直接切回列表
                                        if key.code == KeyCode::Esc {
                                            // 检查编辑器是否在 Normal 模式（通过先处理 Esc，
                                            // 如果返回 Continue 说明已经在 Normal 模式）
                                            let input =
                                                crate::tui::editor_core::vim::Input::from_keycode(
                                                    key.code,
                                                    key.modifiers,
                                                );
                                            let action = editor.handle_input(&input);
                                            match action {
                                                crate::tui::editor_core::EditorAction::Continue => {
                                                    // Esc 在 Normal 模式无效果 → 切回列表
                                                    if app.editor_dirty {
                                                        app.save_editor_content();
                                                    }
                                                    app.focus = Focus::Tree;
                                                }
                                                crate::tui::editor_core::EditorAction::Submit(
                                                    _,
                                                ) => {
                                                    app.save_editor_content();
                                                    app.focus = Focus::Tree;
                                                }
                                                crate::tui::editor_core::EditorAction::Cancel => {
                                                    app.focus = Focus::Tree;
                                                }
                                                crate::tui::editor_core::EditorAction::Save(_) => {
                                                    app.save_editor_content();
                                                }
                                            }
                                        } else {
                                            let input =
                                                crate::tui::editor_core::vim::Input::from_keycode(
                                                    key.code,
                                                    key.modifiers,
                                                );
                                            let action = editor.handle_input(&input);
                                            match action {
                                                crate::tui::editor_core::EditorAction::Submit(
                                                    _,
                                                ) => {
                                                    app.save_editor_content();
                                                    app.focus = Focus::Tree;
                                                }
                                                crate::tui::editor_core::EditorAction::Cancel => {
                                                    app.focus = Focus::Tree;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        AppMode::Adding => {
                            handle_input_mode(&mut app, key);
                            if let Some(title) = app.pending_edit_title.take() {
                                // 新建笔记：创建文件并加载到编辑器
                                let file_path =
                                    crate::command::notebook::app::io::note_file_path(&title);
                                if let Some(parent) = file_path.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                let _ = std::fs::write(&file_path, "");
                                app.reload();
                                // 选中新建的笔记
                                if let Some(pos) = app.flat_entries.iter().position(|e| {
                                    matches!(&e.kind, FlatEntryKind::File { note_index } if app.notes[*note_index].path == title)
                                }) {
                                    app.state.select(Some(pos));
                                    app.load_editor_for_selected();
                                }
                                app.focus = Focus::Editor;
                            }
                        }
                        AppMode::Renaming | AppMode::Search | AppMode::Mkdir | AppMode::Mv => {
                            handle_input_mode(&mut app, key);
                        }
                        AppMode::ConfirmDelete => handle_confirm_delete(&mut app, key),
                        AppMode::CommandPopup => handle_command_popup_mode(&mut app, key),
                        AppMode::RatioInput => handle_ratio_input_mode(&mut app, key),
                    }
                }
                Event::Mouse(mouse) => {
                    let frame_area = terminal.get_frame().area();
                    let layout = compute_mouse_layout(frame_area, &app);
                    let editor_area = layout.preview_area.unwrap_or_default();
                    handle_mouse_event(&mut app, mouse, &layout, editor_area);

                    // 消费后续鼠标事件
                    while event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
                        if let Ok(Event::Mouse(m)) = event::read() {
                            handle_mouse_event(&mut app, m, &layout, editor_area);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture
    )?;
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
