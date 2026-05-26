pub mod manager;
pub mod task;
pub mod tool;

// Re-export public API
pub use manager::{BackgroundManager, build_running_summary};
pub use task::{BgNotification, BgTask};
pub use tool::TaskOutputTool;
