//! Notebook 命令处理模块。
//!
//! 模块拆分：
//! - `cli_commands` — CLI 子命令处理（list/search/delete/open/rename/mkdir/mv 等）
//! - `tui_loop` — TUI 主事件循环
//! - `mouse` — 鼠标事件处理

pub mod cli_commands;
pub mod mouse;
pub mod tui_loop;

pub use cli_commands::handle_notebook;
