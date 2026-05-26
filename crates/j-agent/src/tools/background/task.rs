use std::io::Write;
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use std::time::Instant;

/// 后台任务状态
pub struct BgTask {
    pub task_id: String,
    pub command: String,
    pub status: String, // "running" | "completed" | "error" | "timeout" | "dead"
    /// 共享输出缓冲区，reader 线程实时写入，查询时可直接读取中间输出
    pub output_buffer: Arc<Mutex<String>>,
    pub result: Option<String>,
    /// 任务启动时间，用于计算已运行时长
    pub started_at: Instant,
    /// 子进程 PID，用于存活检测（仅 shell 后台任务有值，SubAgent 后台无子进程）
    pub child_pid: Option<u32>,
    /// 线程类任务的存活标记（SubAgent 等非进程任务使用 AtomicBool 标记存活）
    pub is_thread_running: Option<Arc<AtomicBool>>,
    /// PTY writer 句柄（仅交互式会话有值），用于 stdin 写入
    pub pty_writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
}

/// 后台任务完成通知
#[derive(Debug)]
pub struct BgNotification {
    pub task_id: String,
    pub command: String,
    pub status: String,
    pub result: String,
}
