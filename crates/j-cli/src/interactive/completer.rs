//! REPL 补全模块入口
//!
//! 子模块：
//! - `completion_rules` — 参数类型枚举 (ArgHint) 与命令参数补全规则表
//! - `engine` — 补全器 (CopilotCompleter)、历史提示器、高亮器、Helper 及文件路径补全

pub mod completion_rules;
pub mod engine;

// 重新导出外部消费者实际使用的类型
pub use engine::CopilotHelper;
