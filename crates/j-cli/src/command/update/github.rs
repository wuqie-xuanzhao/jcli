//! GitHub 更新主流程

use super::auth::get_github_auth_token;
use super::fallback::perform_update_fallback;
#[allow(unused_imports)]
use super::indicator::install_indicator_from_release;
use super::macos::fix_codesign_and_quarantine;
use super::restart::restart_self;
use crate::constants::VERSION;
use colored::Colorize;

/// 从 GitHub Releases 更新
pub(crate) fn handle_github_update(check_only: bool, interactive: bool) {
    println!("{}", "检测到 GitHub Release 安装方式".green());
    println!("当前版本: {}", VERSION.cyan());

    if check_only {
        check_for_update();
    } else {
        perform_update(interactive);
    }
}

/// 检查是否有新版本
fn check_for_update() {
    println!("{}", "正在检查更新...".yellow());

    let auth_token = get_github_auth_token();
    if auth_token.is_some() {
        println!("{}", "使用 GitHub 认证...".dimmed());
    }

    let mut binding = self_update::backends::github::ReleaseList::configure();
    let mut binding = binding.repo_owner("LingoJack").repo_name("j");

    if let Some(ref token) = auth_token {
        binding = binding.auth_token(token);
    }

    match binding.build() {
        Ok(release_list) => match release_list.fetch() {
            Ok(releases) => {
                if let Some(latest) = releases.first() {
                    let latest_version = latest.version.trim_start_matches('v');
                    println!("最新版本: {}", latest_version.cyan());

                    if latest_version == VERSION {
                        println!("{}", "已是最新版本".green());
                    } else {
                        println!("{}", "发现新版本！运行 'j update' 进行更新".yellow());
                    }
                } else {
                    println!("{}", "未找到发布版本".red());
                }
            }
            Err(e) => {
                println!("{} {}", "检查更新失败:".red(), e);
                println!("请尝试手动更新:");
                #[cfg(unix)]
                println!(
                    "  curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh"
                );
                #[cfg(windows)]
                println!(
                    "  irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex"
                );
            }
        },
        Err(e) => {
            println!("{} {}", "配置更新源失败:".red(), e);
        }
    }
}

/// 执行更新
fn perform_update(interactive: bool) {
    println!("{}", "正在更新...".yellow());

    // 根据当前架构确定 target 名称（匹配 GitHub Release 资产命名）
    // 资产命名格式: j-darwin-arm64.tar.gz, j-darwin-x64.tar.gz
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    let target = "darwin-arm64";

    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    let target = "darwin-x64";

    #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
    let target = "windows-x64";

    #[cfg(all(target_arch = "aarch64", target_os = "windows"))]
    let target = "windows-arm64";

    #[cfg(not(any(
        all(target_arch = "aarch64", target_os = "macos"),
        all(target_arch = "x86_64", target_os = "macos"),
        all(target_arch = "x86_64", target_os = "windows"),
        all(target_arch = "aarch64", target_os = "windows")
    )))]
    let target = {
        println!("{}", "当前平台暂不支持自动更新，请手动更新".red());
        return;
    };

    // 检查是否已经有 root 权限
    // SAFETY: libc::getuid() 是只读系统调用，无副作用，线程安全
    #[cfg(unix)]
    let is_root = unsafe { libc::getuid() == 0 };

    #[cfg(not(unix))]
    let is_root = false;

    // 如果已经有 root 权限，直接执行更新
    if is_root {
        perform_update_internal(target, interactive);
        return;
    }

    // 检查是否有权限写入目标目录
    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("{} {}", "无法获取当前可执行文件路径:".red(), e);
            return;
        }
    };

    let exe_dir = match exe_path.parent() {
        Some(d) => d,
        None => {
            println!("{}", "无法获取可执行文件所在目录".red());
            return;
        }
    };

    // 尝试创建临时文件来验证实际的写入权限
    let can_actually_write = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(exe_dir.join(".j_write_test"))
        .map(|_| {
            let _ = std::fs::remove_file(exe_dir.join(".j_write_test"));
            true
        })
        .unwrap_or(false);

    if can_actually_write {
        // 有写入权限，直接执行更新
        perform_update_internal(target, interactive);
        return;
    }

    // 没有写入权限，需要提升权限
    #[cfg(target_os = "macos")]
    {
        println!(
            "{}",
            "需要管理员权限来更新 j（安装目录需要 root 权限）".yellow()
        );
        println!("{}", "正在请求管理员权限...".cyan());

        // 使用 osascript 弹出图形化授权对话框
        let exe_str = exe_path.to_string_lossy();
        let script = format!(
            r#"do shell script "{} update" with administrator privileges"#,
            exe_str
        );

        let result = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status();

        match result {
            Ok(status) if status.success() => {
                println!("{}", "更新完成！".green());
            }
            Ok(status) => {
                println!(
                    "{} 退出码: {}",
                    "更新失败".red(),
                    status.code().unwrap_or(-1)
                );
                println!("请尝试手动更新:");
                println!("  {}", "sudo j update".cyan());
            }
            Err(e) => {
                println!("{} {}", "请求权限失败:".red(), e);
                println!("请尝试手动更新:");
                println!("  {}", "sudo j update".cyan());
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        println!("{}", "需要管理员权限来更新 j".yellow());
        println!("请以管理员身份运行: {}", "j update".cyan());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        println!(
            "{}",
            "需要管理员权限来更新 j（安装目录需要 root 权限）".yellow()
        );
        println!("请尝试手动更新:");
        println!("  {}", "sudo j update".cyan());
    }
}

/// 内部更新逻辑（假设已有权限）
fn perform_update_internal(target: &str, interactive: bool) {
    let auth_token = get_github_auth_token();
    if auth_token.is_some() {
        println!("{}", "使用 GitHub 认证...".dimmed());
    }

    let mut binding = self_update::backends::github::Update::configure();
    let mut binding = binding
        .repo_owner("LingoJack")
        .repo_name("jcli")
        .bin_name("j")
        .show_download_progress(true)
        .current_version(VERSION)
        .target(target);

    if let Some(ref token) = auth_token {
        binding = binding.auth_token(token);
    }

    match binding.build() {
        Ok(updater) => match updater.update() {
            Ok(status) => {
                // 修复 macOS 代码签名（替换后的二进制文件需要重签）
                if let Ok(exe_path) = std::env::current_exe() {
                    fix_codesign_and_quarantine(&exe_path);
                }
                println!(
                    "{} {}",
                    "更新成功！".green(),
                    format!("版本: {}", status.version()).cyan()
                );
                // 尝试同步安装 j-indicator（仅 macOS）
                #[cfg(target_os = "macos")]
                install_indicator_from_release(status.version());
                if interactive {
                    restart_self();
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                // 多种错误情况都尝试使用备用方案更新
                // - 403/rate limit: GitHub API 限流
                // - decoding/TLS/IoError: Windows 上 rustls 可能遇到的问题
                // - certificate/ssl: 证书验证失败
                // - ReqwestError/sending request: 网络不通或代理问题
                let should_fallback = err_str.contains("403")
                    || err_str.contains("rate limit")
                    || err_str.contains("decoding")
                    || err_str.contains("IoError")
                    || err_str.contains("certificate")
                    || err_str.contains("ssl")
                    || err_str.contains("TLS")
                    || err_str.contains("connection")
                    || err_str.contains("ReqwestError")
                    || err_str.contains("sending request");

                if should_fallback {
                    println!("{} {}", "更新失败:".red(), e);
                    println!("{}", "尝试使用备用方式更新...".yellow());
                    perform_update_fallback(target, interactive);
                } else {
                    println!("{} {}", "更新失败:".red(), e);
                    println!("请尝试手动更新:");
                    #[cfg(unix)]
                    println!(
                        "  curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh"
                    );
                    #[cfg(windows)]
                    println!(
                        "  irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex"
                    );
                }
            }
        },
        Err(e) => {
            println!("{} {}", "配置更新失败:".red(), e);
        }
    }
}
