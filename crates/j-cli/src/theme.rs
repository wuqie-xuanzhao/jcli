//! 主题模块：结构体定义、JSON 解析、运行时实现

mod impls;
mod parse;
mod types;

// 公共类型 re-export——外部 `use crate::theme::{Theme, ThemeName}` 不受影响
pub use types::{Theme, ThemeName};
