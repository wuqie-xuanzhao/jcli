//! 主题模块：结构体定义、JSON 解析、运行时实现

mod impls;
mod parse;
mod types;

// Re-export BorderStyle related functions from j-tui
pub use j_tui::editor_core::theme::init_border_style;

// 公共类型 re-export
pub use types::{Theme, ThemeName};
