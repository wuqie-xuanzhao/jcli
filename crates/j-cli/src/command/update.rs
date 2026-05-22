//! update 命令模块入口
//!
//! 公共 API: `handle_update(check_only, interactive)`

mod auth;
mod cargo_update;
mod fallback;
mod feature_select;
mod github;
mod indicator;
mod macos;
mod restart;

use crate::constants::INSTALL_SOURCE;
use cargo_update::show_unknown_source_hint;
use github::handle_github_update;

/// 处理 update 命令
pub fn handle_update(check_only: bool, interactive: bool) {
    match INSTALL_SOURCE {
        "github" => handle_github_update(check_only, interactive),
        "cargo" => cargo_update::handle_cargo_update(check_only, interactive),
        _ => show_unknown_source_hint(interactive),
    }
}
