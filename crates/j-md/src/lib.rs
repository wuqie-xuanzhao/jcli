//! Shared Markdown IR and parser
//!
//! Provides a platform-independent Markdown intermediate representation (IR)
//! and a parser that converts Markdown text into structured `ParsedDocument`.
//! No terminal or GUI dependencies — suitable for both TUI and GUI consumers.

pub mod ir;
pub mod parser;
pub mod util;

pub use ir::*;
pub use parser::parse_markdown;
pub use util::{char_width, display_width};
