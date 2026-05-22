//! Chat 主界面绘制入口
//!
//! 使用 `chat.rs` + `chat/` 模式拆分为 4 个子模块：
//! - `selection` — 鼠标选区功能
//! - `render_text` — 文字渲染 pass
//! - `render_image` — 图片渲染 pass
//! - `render_messages` — 消息列表绘制

mod render_image;
mod render_messages;
mod render_text;
mod selection;

pub use selection::{copy_selection_to_clipboard, extract_selection_text, screen_to_text_pos};

pub use render_messages::draw_messages;

use super::archive::{draw_archive_confirm, draw_archive_list};
use super::config::draw_config_screen;
use super::popup;
use super::title_bar;
use crate::command::chat::app::ChatMode;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Clear},
};

/// 绘制 Chat 主界面：标题栏、消息区、输入区、提示栏及各类弹窗覆盖层
pub fn draw_chat_ui(f: &mut ratatui::Frame, app: &mut crate::command::chat::app::ChatApp) {
    let size = f.area();

    // 整体背景：先清除旧内容，再填充背景色。
    // Windows 上 crossterm 差异缓冲区可能不清理旧内容，导致切换模式时残留上一帧的字符。
    f.render_widget(Clear, size);
    let bg = Block::default().style(Style::default().bg(app.ui.theme.bg_primary));
    f.render_widget(bg, size);

    // 动态标题栏高度：顶部分割线(1) + 状态行(1) + 可选分割线(1) + 可选 teammate 行 + 可选 subagent 行
    let has_teammates = app
        .teammate_manager
        .lock()
        .map(|m| !m.teammates.is_empty())
        .unwrap_or(false);
    let has_subagents = !app.sub_agent_tracker.display_snapshots().is_empty();
    let title_height = title_bar::calc_title_height(app);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_height), // 标题栏（顶部分割线 + 内容行 + 可选 teammate 行）
            Constraint::Min(5),               // 消息区
            Constraint::Length(5),            // 输入区
            Constraint::Length(1),            // 操作提示栏（始终可见）
        ])
        .split(size);

    // ========== 标题栏 ==========
    title_bar::draw_title_bar(f, chunks[0], app, has_teammates, has_subagents);

    // ========== 消息区 ==========
    match app.ui.mode {
        ChatMode::Help => super::help::draw_help(f, chunks[1], app),
        ChatMode::SelectModel => super::selector::draw_model_selector(f, chunks[1], app),
        ChatMode::SelectTheme => super::selector::draw_theme_selector(f, chunks[1], app),
        ChatMode::Config => draw_config_screen(f, chunks[1], app),
        ChatMode::ArchiveConfirm => draw_archive_confirm(f, chunks[1], app),
        ChatMode::ArchiveList => draw_archive_list(f, chunks[1], app),
        // 这些模式的主区域均显示消息列表
        ChatMode::Chat
        | ChatMode::Browse
        | ChatMode::ToolConfirm
        | ChatMode::AgentPermConfirm
        | ChatMode::PlanApprovalConfirm => draw_messages(f, chunks[1], app),
    }

    // ========== 输入区 ==========
    super::input::draw_input(f, chunks[2], app);

    // ========== 底部操作提示栏（始终可见）==========
    super::hint::draw_hint_bar(f, chunks[3], app);

    // ========== Toast 弹窗覆盖层（右上角）==========
    super::hint::draw_toast(f, size, app);

    // ========== @ 补全弹窗覆盖层 ==========
    if app.ui.at_popup_active {
        popup::draw_at_popup(f, chunks[2], app);
    }

    // ========== 文件补全弹窗覆盖层 ==========
    if app.ui.file_popup_active {
        popup::draw_file_popup(f, chunks[2], app);
    }

    // ========== 技能补全弹窗覆盖层 ==========
    if app.ui.skill_popup_active {
        popup::draw_skill_popup(f, chunks[2], app);
    }

    // ========== 命令补全弹窗覆盖层 ==========
    if app.ui.command_popup_active {
        popup::draw_command_popup(f, chunks[2], app);
    }

    // ========== / 斜杠命令弹窗覆盖层 ==========
    if app.ui.slash_popup_active {
        popup::draw_slash_popup(f, chunks[2], app);
    }

    // ========== 右键上下文菜单覆盖层 ==========
    super::context_menu::draw_context_menu(f, app);
}
