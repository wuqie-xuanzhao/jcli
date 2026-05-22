//! CLI 命令处理函数。
//!
//! 处理 `j notebook` 和 `j md` 命令的子命令逻辑。

use crate::command::chat::storage::load_agent_config;
use crate::command::notebook::app::{
    edit_note_with_editor, load_notes, note_file_path, notebook_dir,
};
use crate::constants::{notebook_action, shell};
use crate::theme::Theme;
use crate::util::fuzzy;
use colored::Colorize;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::process::Command;

/// 默认临时笔记名前缀
const TEMP_NOTE_PREFIX: &str = "temp_note_";

/// 处理 notebook/md 命令入口
///
/// `from_notebook_cmd=true` 表示由 `j notebook` 调用，无参数时进入 TUI 列表；
/// `from_notebook_cmd=false` 表示由 `j md` 调用，无参数时编辑默认临时笔记。
pub fn handle_notebook(args: &[String], from_notebook_cmd: bool) {
    // 优先检测 stdin 管道输入：非终端时读取并渲染 Markdown 到 stdout
    if !std::io::stdin().is_terminal() {
        handle_stdin_render();
        return;
    }

    if args.is_empty() {
        if from_notebook_cmd {
            super::tui_loop::run_notebook_tui();
        } else {
            handle_edit_default_temp_note();
        }
        return;
    }

    let first = args[0].as_str();
    match first {
        f if f == notebook_action::LIST => handle_list(),
        f if f == notebook_action::SEARCH => {
            if let Some(keyword) = args.get(1) {
                handle_search(keyword);
            } else {
                crate::error!("用法: md search <关键词>");
            }
        }
        f if f == notebook_action::DELETE => {
            if let Some(title) = args.get(1) {
                handle_delete(title);
            } else {
                crate::error!("用法: md delete <笔记路径>");
            }
        }
        f if f == notebook_action::OPEN => handle_open(),
        f if f == notebook_action::RENAME => {
            if args.len() >= 3 {
                handle_rename(&args[1], &args[2]);
            } else {
                crate::error!("用法: md rename <旧路径> <新路径>");
            }
        }
        f if f == notebook_action::MKDIR => {
            if let Some(name) = args.get(1) {
                handle_mkdir(name);
            } else {
                crate::error!("用法: md mkdir <目录名>");
            }
        }
        f if f == notebook_action::MV => {
            if args.len() >= 3 {
                handle_mv(&args[1], &args[2]);
            } else {
                crate::error!("用法: md mv <源路径> <目标路径>");
            }
        }
        _ => {
            let joined = args.join(" ");
            if is_file_path(&joined) {
                edit_file_with_editor(&joined);
            } else {
                edit_note_with_editor(&joined);
            }
        }
    }
}

/// 从 stdin 读取 Markdown 文本，渲染为 ANSI 彩色输出到 stdout
fn handle_stdin_render() {
    use std::io::Read;

    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("读取 stdin 失败: {e}");
        std::process::exit(1);
    }
    if input.trim().is_empty() {
        return;
    }
    crate::util::md_render::render_md(&input);
}

/// 无参数时编辑默认临时笔记：自动选取 temp_note_{N}.md 中第一个不存在的编号
fn handle_edit_default_temp_note() {
    let dir = notebook_dir();
    let _ = fs::create_dir_all(&dir);

    let index = find_next_temp_note_index(&dir);
    let note_name = format!("{}{}", TEMP_NOTE_PREFIX, index);
    edit_note_with_editor(&note_name);
}

/// 找到下一个可用的临时笔记编号（从 0 开始递增，找到第一个不存在的）
fn find_next_temp_note_index(dir: &std::path::Path) -> u32 {
    let mut index = 0;
    loop {
        let file_name = format!("{}{}.md", TEMP_NOTE_PREFIX, index);
        if !dir.join(&file_name).exists() {
            return index;
        }
        index += 1;
    }
}

fn is_file_path(s: &str) -> bool {
    if s.starts_with('~') || s.contains('.') {
        return true;
    }
    if s.contains('/') {
        let potential_note = note_file_path(s);
        if potential_note.starts_with(notebook_dir()) {
            return false;
        }
        return true;
    }
    false
}

fn edit_file_with_editor(file_str: &str) {
    let expanded = expand_tilde(file_str);
    let path = std::path::PathBuf::from(&expanded);

    let (content, is_new_file) = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(c) => (c, false),
            Err(e) => {
                crate::error!("读取文件失败: {} - {}", path.display(), e);
                return;
            }
        }
    } else {
        (String::new(), true)
    };

    let theme = Theme::from_name(&load_agent_config().theme);

    let title = if is_new_file {
        format!("{} (新文件)", path.display())
    } else {
        path.display().to_string()
    };

    match crate::tui::editor_markdown::open_markdown_editor(&title, &content, &theme) {
        Ok((Some(new_content), _)) => {
            if is_new_file || new_content != content {
                if let Some(parent) = path.parent()
                    && !parent.exists()
                    && let Err(e) = std::fs::create_dir_all(parent)
                {
                    crate::error!("创建目录失败: {} - {}", parent.display(), e);
                    return;
                }

                match std::fs::write(&path, &new_content) {
                    Ok(()) => crate::info!("文件已保存: {}", path.display()),
                    Err(e) => crate::error!("保存文件失败: {} - {}", path.display(), e),
                }
            } else {
                crate::info!("内容未变化，跳过保存");
            }
        }
        Ok((None, _)) => crate::info!("已取消编辑"),
        Err(e) => crate::error!("编辑器启动失败: {}", e),
    }
}

fn expand_tilde(path: &str) -> String {
    if (path == "~" || path.starts_with("~/"))
        && let Some(home) = dirs::home_dir()
    {
        if path == "~" {
            home.display().to_string()
        } else {
            format!("{}{}", home.display(), &path[1..])
        }
    } else {
        path.to_string()
    }
}

fn handle_list() {
    let notes = load_notes();
    if notes.is_empty() {
        crate::info!("📓 notebook 为空");
        return;
    }

    println!("{}", format!("📓 共 {} 篇笔记：", notes.len()).bold());
    for note in &notes {
        println!(
            "  {}  {}",
            note.path,
            crate::command::notebook::app::format_time(note.mtime).dimmed()
        );
    }
}

fn handle_search(keyword: &str) {
    let notes = load_notes();
    if notes.is_empty() {
        crate::info!("📓 notebook 为空");
        return;
    }

    let mut found = false;
    for note in &notes {
        let file_path = note_file_path(&note.path);
        if let Ok(content) = fs::read_to_string(&file_path)
            && (fuzzy::fuzzy_match(&content, keyword) || fuzzy::fuzzy_match(&note.path, keyword))
        {
            if !found {
                println!("{}", format!("🔍 搜索 \"{}\" 的结果：", keyword).bold());
                found = true;
            }
            println!("\n  {}", note.path.cyan().bold());
            for (line_num, line) in content.lines().enumerate() {
                if fuzzy::fuzzy_match(line, keyword) {
                    println!(
                        "    {}: {}",
                        format!("L{}", line_num + 1).dimmed(),
                        line.trim()
                    );
                }
            }
        }
    }

    if !found {
        crate::info!("未找到包含 \"{}\" 的笔记", keyword);
    }
}

fn handle_delete(title: &str) {
    let file_path = note_file_path(title);
    if !file_path.exists() {
        let notes = load_notes();
        let matched: Vec<&str> = notes
            .iter()
            .map(|n| n.path.as_str())
            .filter(|path| fuzzy::fuzzy_match(path, title))
            .collect();

        if matched.is_empty() {
            crate::error!("未找到笔记: {}", title);
        } else {
            println!("未找到精确匹配，你是否要删除以下笔记？");
            for path in &matched {
                println!("  - {}", path);
            }
            crate::info!("请使用精确路径: md delete <路径>");
        }
        return;
    }

    print!("确认删除笔记 \"{}\"？(y/N): ", title);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return;
    }
    if input.trim().to_lowercase() == "y" {
        match fs::remove_file(&file_path) {
            Ok(()) => {
                crate::command::notebook::app::cleanup_empty_dirs();
                crate::info!("已删除笔记: {}", title);
            }
            Err(e) => crate::error!("删除失败: {}", e),
        }
    } else {
        crate::info!("已取消删除");
    }
}

fn handle_open() {
    let dir = notebook_dir();
    let path = dir.to_string_lossy().to_string();
    let os = std::env::consts::OS;
    let result = if os == shell::MACOS_OS {
        Command::new("open").arg(&path).status()
    } else if os == shell::WINDOWS_OS {
        Command::new(shell::WINDOWS_CMD)
            .args([shell::WINDOWS_CMD_FLAG, "start", "", &path])
            .status()
    } else {
        Command::new("xdg-open").arg(&path).status()
    };

    if let Err(e) = result {
        crate::error!("打开目录失败: {}", e);
    }
}

fn handle_rename(old_name: &str, new_name: &str) {
    let old_path = note_file_path(old_name);
    let new_path = note_file_path(new_name);

    if !old_path.exists() {
        crate::error!("未找到笔记: {}", old_name);
        return;
    }
    if new_path.exists() {
        crate::error!("目标笔记已存在: {}", new_name);
        return;
    }

    if let Some(parent) = new_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::rename(&old_path, &new_path) {
        Ok(()) => {
            crate::command::notebook::app::cleanup_empty_dirs();
            crate::info!("已重命名: {} → {}", old_name, new_name);
        }
        Err(e) => crate::error!("重命名失败: {}", e),
    }
}

fn handle_mkdir(name: &str) {
    let dir_path = notebook_dir().join(name);
    if dir_path.exists() {
        crate::error!("目录已存在: {}", name);
        return;
    }
    match fs::create_dir_all(&dir_path) {
        Ok(()) => crate::info!("已创建目录: {}", name),
        Err(e) => crate::error!("创建目录失败: {}", e),
    }
}

fn handle_mv(source: &str, target: &str) {
    let old_path = note_file_path(source);
    let new_path = note_file_path(target);
    if !old_path.exists() {
        crate::error!("源笔记不存在: {}", source);
        return;
    }
    if new_path.exists() {
        crate::error!("目标笔记已存在: {}", target);
        return;
    }
    if let Some(parent) = new_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::rename(&old_path, &new_path) {
        Ok(()) => {
            crate::command::notebook::app::cleanup_empty_dirs();
            crate::info!("已移动: {} → {}", source, target);
        }
        Err(e) => crate::error!("移动失败: {}", e),
    }
}
