//! 配置界面更新逻辑模块
//!
//! 包含配置界面所有交互处理，拆分为以下子模块：
//! - `config_navigation`: 配置界面导航、Tab 切换、字段选择
//! - `config_edit`: 配置字段编辑、Provider 管理、Enter 操作
//! - `config_toggle_menu`: Tools/Skills/Commands/Hooks 启用禁用管理
//! - `config_select`: 各种列表选择操作

mod config_edit;
mod config_navigation;
mod config_select;
mod config_toggle_menu;
