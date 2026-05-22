//! GitHub 认证

/// 尝试获取 GitHub 认证 token
/// 优先级: GITHUB_TOKEN 环境变量 > gh auth token
pub(crate) fn get_github_auth_token() -> Option<String> {
    // 方法1: 检查 GITHUB_TOKEN 环境变量
    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.is_empty()
    {
        return Some(token);
    }

    // 方法2: 尝试使用 gh auth token
    if let Ok(output) = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        && output.status.success()
    {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }

    None
}
