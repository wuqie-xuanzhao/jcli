//! Self-contained Markdown editor core
//!
//! Editor, Vim keybindings, text buffer, search, wrap engine, and theme.
//! Does not depend on the chat subsystem's `Theme`.

pub mod editor;
pub mod history;
pub mod markdown_cache;
pub mod renderer;
pub mod search;
pub mod text_buffer;
pub mod theme;
pub mod vim;
pub mod wrap_engine;

pub use theme::{EditorTheme, HighlightFn};
pub use editor::{CursorPolicy, MarkdownEditor, MarkdownEditorOpts, ThemeGalleryItem, EditorAction};
pub use editor::{open_markdown_editor, open_markdown_editor_on_terminal, open_markdown_editor_with_content};
