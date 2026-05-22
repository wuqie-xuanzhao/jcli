use crate::command::chat::remote::protocol::FileEntry;

/// 远程文件/终端操作（静态方法，不依赖 ChatApp 实例）
pub struct RemoteOps;

impl RemoteOps {
    /// 列出指定路径下的文件和目录（隐藏文件除外）
    pub fn handle_file_list(path: &str) -> Vec<FileEntry> {
        let dir = if path.is_empty() { "." } else { path };
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(dir) {
            let mut dirs: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
            dirs.sort_by(|a, b| {
                let a_dir = a.file_type().map(|t| !t.is_dir()).unwrap_or(true);
                let b_dir = b.file_type().map(|t| !t.is_dir()).unwrap_or(true);
                b_dir
                    .cmp(&a_dir)
                    .then_with(|| a.file_name().cmp(&b.file_name()))
            });
            for entry in dirs {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let modified = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_default();
                entries.push(FileEntry {
                    name,
                    is_dir,
                    size,
                    modified,
                });
            }
        }
        entries
    }

    /// 读取文件内容，返回 (内容, 错误信息)
    pub fn handle_file_read(path: &str) -> (String, Option<String>) {
        match std::fs::read_to_string(path) {
            Ok(content) => (content, None),
            Err(e) => (String::new(), Some(e.to_string())),
        }
    }

    /// 写入文件内容，返回 (是否成功, 错误信息)
    pub fn handle_file_write(path: &str, content: &str) -> (bool, Option<String>) {
        match std::fs::write(path, content) {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        }
    }

    /// 在 shell 中执行命令，返回 (输出内容, 退出码)
    pub fn handle_terminal_exec(command: &str) -> (String, Option<i32>) {
        use std::process::Command;
        let output = Command::new("sh").arg("-c").arg(command).output();
        match output {
            Ok(out) => {
                let mut result = String::new();
                if !out.stdout.is_empty() {
                    result.push_str(&String::from_utf8_lossy(&out.stdout));
                }
                if !out.stderr.is_empty() {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(&String::from_utf8_lossy(&out.stderr));
                }
                let exit_code = out.status.code();
                (result, exit_code)
            }
            Err(e) => (e.to_string(), None),
        }
    }
}

// 保留 ChatApp 上的静态方法包装，方便调用方
impl super::ChatApp {
    /// 列出指定路径下的文件和目录（委托 RemoteOps）
    pub fn handle_file_list(path: &str) -> Vec<FileEntry> {
        RemoteOps::handle_file_list(path)
    }

    /// 读取文件内容（委托 RemoteOps）
    pub fn handle_file_read(path: &str) -> (String, Option<String>) {
        RemoteOps::handle_file_read(path)
    }

    /// 写入文件内容（委托 RemoteOps）
    pub fn handle_file_write(path: &str, content: &str) -> (bool, Option<String>) {
        RemoteOps::handle_file_write(path, content)
    }

    /// 在 shell 中执行命令（委托 RemoteOps）
    pub fn handle_terminal_exec(command: &str) -> (String, Option<i32>) {
        RemoteOps::handle_terminal_exec(command)
    }
}
