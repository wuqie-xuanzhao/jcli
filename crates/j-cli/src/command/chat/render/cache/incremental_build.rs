//! 增量渲染模块
//!
//! 按职责拆分为以下子模块：
//! - `history_build` — 历史消息缓存构建（全量增量构建）
//! - `streaming_build` — 流式内容渲染（仅重建流式 + 共享辅助函数）

mod history_build;
mod streaming_build;

// ── Re-export 公共 API（保持外部引用路径不变）──
pub use history_build::build_message_lines_incremental;
pub use streaming_build::rebuild_streaming_only;
