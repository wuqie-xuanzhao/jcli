//! Markdown parser, renderer, and theme abstraction
//!
//! Provides a markdown-to-terminal rendering pipeline that is independent
//! of the chat Theme. Callers supply an `EditorTheme` (or any type implementing
//! `MdStyle`) to control colors.

pub mod highlight;
pub mod ir;
pub mod parser;
pub mod render;
pub mod theme;

#[cfg(feature = "image")]
pub mod image_cache;
#[cfg(feature = "image")]
pub mod image_loader;

pub use parser::markdown_to_lines;
