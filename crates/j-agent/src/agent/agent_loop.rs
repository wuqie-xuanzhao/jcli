pub mod compact;
pub mod loop_stages;
pub mod tool_execution;

// Re-export the public API
pub use loop_stages::{MainAgentLoopParams, run_main_agent_loop};
