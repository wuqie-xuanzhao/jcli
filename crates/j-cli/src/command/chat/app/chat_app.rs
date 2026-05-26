mod init_hooks;
mod model_mgr;
mod remote;
mod remote_ops;
mod tool_ops;
mod update;
mod update_config;
mod update_misc;
mod update_session;
mod update_tool_interact;

use super::agent_handle::MainAgentHandle;
use super::chat_state::ChatState;
use super::tool_executor::ToolExecutor;
use super::types::AskRequest;
use super::ui_state::{ChatMode, CommandsMode, ConfigTab, UIState};
use crate::command::chat::agent_md;
use crate::command::chat::infra::command::{self, CommandSource};
use crate::command::chat::infra::hook::{HookContext, HookEvent, HookManager};
use crate::command::chat::infra::sandbox::Sandbox;
use crate::command::chat::infra::skill;
use crate::command::chat::input::file_index::FileIndex;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::permission::queue::PermissionQueue;
use crate::command::chat::storage::{
    ChatMessage, ModelProvider, load_agent_config, memory_path, save_memory, save_soul,
    save_system_prompt, soul_path, system_prompt_path,
};
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::derived_shared::{
    AgentContextConfig, DerivedAgentShared, SubAgentTracker,
};
use crate::command::chat::tools::plan::PlanApprovalQueue;
use crate::command::chat::tools::task::TaskManager;
use crate::command::chat::tools::todo::TodoManager;
use crate::command::chat::tools::{ToolDefinitionParams, ToolRegistry};
use crate::constants::TOAST_DURATION_SECS;
use crate::markdown::image_cache::ImageCache;
use crate::theme::Theme;
use crate::tui::editor_core::text_buffer::TextBuffer;
use crate::util::safe_lock;
use ratatui::widgets::ListState;
use std::sync::{Arc, Mutex, mpsc};

// ========== 主应用结构体 ==========

/// TUI 应用状态（组合结构）
pub struct ChatApp {
    /// 前端 UI 状态
    pub ui: UIState,
    /// 后端数据状态
    pub state: ChatState,
    /// 工具执行器
    pub tool_executor: ToolExecutor,
    /// 主 Agent 生命周期句柄（存在时表示有进行中的请求）
    pub main_agent: Option<MainAgentHandle>,
    /// 工具注册表
    pub tool_registry: Arc<ToolRegistry>,
    /// .jcli/ 权限配置
    pub jcli_config: Arc<JcliConfig>,
    /// 后台任务管理器
    pub background_manager: Arc<BackgroundManager>,
    /// Task 管理器（由内置 hook 和工具通过 Arc 引用使用）
    #[allow(dead_code)]
    pub task_manager: Arc<TaskManager>,
    /// Todo 管理器
    pub todo_manager: Arc<TodoManager>,
    /// ask 工具响应发送通道
    pub ask_response_tx: Option<mpsc::Sender<String>>,
    /// ask 工具请求接收通道
    pub ask_request_rx: Option<mpsc::Receiver<AskRequest>>,
    /// Hook 管理器
    pub hook_manager: Arc<Mutex<HookManager>>,
    /// 安全沙箱（限制工具操作路径范围）
    pub sandbox: Sandbox,
    /// 本次会话 ID（启动时生成，对应 sessions/{id}.jsonl）
    pub session_id: String,
    /// 与 `DerivedAgentShared` 共享的 session id 槽；切换 session 时用 `switch_session_id` 同步更新。
    pub shared_session_id: Arc<Mutex<String>>,
    /// 已持久化到 JSONL 的消息数量（用于增量追加）
    pub persisted_message_count: usize,
    /// 已持久化到 display.jsonl 的消息数量（用于增量追加）
    pub persisted_display_count: usize,
    /// 远程控制 WebSocket 桥接器
    pub ws_bridge: Option<crate::command::chat::remote::bridge::WsBridge>,
    /// 远程客户端是否已连接
    pub remote_connected: bool,
    /// 子 Agent 共用 provider（每次发送请求前更新，AgentTool / TeammateTool 共用）
    pub derived_agent_provider: Arc<Mutex<ModelProvider>>,
    /// 子 Agent 共用 system_prompt（每次发送请求前更新，AgentTool / TeammateTool 共用）
    pub derived_agent_system_prompt: Arc<Mutex<Option<String>>>,
    /// 子 Agent 共用上下文配置快照（每次发送请求前刷新）
    pub derived_agent_context_config: Arc<Mutex<AgentContextConfig>>,
    /// 子 Agent 使用的 disabled_hooks 快照（每次发送请求前刷新）
    pub derived_agent_disabled_hooks: Arc<Mutex<Vec<String>>>,
    /// Agent/Teammate → UI 的显示通道（agent 线程 push，UI 线程 poll len 变化）
    /// 仅用于 UI 渲染，不作为 LLM context 数据源。
    pub display_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// UI 侧已读取到的位置（用于增量检测）
    pub display_read_offset: usize,
    /// Agent/Teammate → LLM context 同步通道
    /// `persist_new_messages` 直接从此通道读取并持久化到 transcript.jsonl。
    /// 只有需要进入 Main Agent LLM context 的消息才写入此通道。
    pub context_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// context 侧已读取到的位置（用于增量检测）
    pub context_read_offset: usize,
    /// Agent 实际使用的上下文 token 估算值（agent 每轮更新，UI 读取显示）
    pub context_tokens: Arc<Mutex<usize>>,
    /// Teammate 管理器（多 agent 协作）
    #[allow(dead_code)]
    pub teammate_manager: Arc<Mutex<TeammateManager>>,
    /// 子 Agent（AgentTool）运行快照追踪器（供 /dump 读取）
    pub sub_agent_tracker: Arc<SubAgentTracker>,
    /// 派生 Agent 权限请求队列（DerivedAgentShared 和 TUI 共享同一个 Arc）
    pub permission_queue: Arc<PermissionQueue>,
    /// Plan 审批请求队列（Teammate ExitPlanMode 和 TUI 共享同一个 Arc）
    pub plan_approval_queue: Arc<PlanApprovalQueue>,
    /// 子 Agent metrics 累加器（SubAgent/Teammate 的 LLM/tool 统计，传递给 AgentLoopSharedState）
    pub sub_agent_metrics: Arc<Mutex<crate::command::chat::tools::derived_shared::SubAgentMetrics>>,
    /// 延迟加载的工具列表（与 DerivedAgentShared 共享，UI 可修改）
    pub deferred_tools: Arc<Mutex<Vec<String>>>,
    /// 本会话通过 LoadTool 加载的 deferred 工具（会话级，不写入用户配置）
    pub session_loaded_deferred: Arc<Mutex<Vec<String>>>,
    /// 会话内已调用技能追踪（LoadSkill 执行时记录，auto_compact 后恢复）
    pub invoked_skills: crate::command::chat::context::compact::InvokedSkillsMap,
    /// 项目文件索引（后台维护，弹窗使用）
    pub file_index: FileIndex,
}

impl ChatApp {
    /// 创建新的 ChatApp 实例，初始化所有子系统和通道
    pub fn new(session_id: String) -> Self {
        let agent_config = load_agent_config();
        // 首次运行：各数据文件不存在时写入默认内容
        if !system_prompt_path().exists() {
            let _ = save_system_prompt(&crate::assets::default_system_prompt());
        }
        if !memory_path().exists() {
            let _ = save_memory(&crate::assets::default_memory());
        }
        if !soul_path().exists() {
            let _ = save_soul(&crate::assets::default_soul());
        }
        if !agent_md::agent_md_path().exists() {
            let _ = std::fs::write(
                agent_md::agent_md_path(),
                crate::assets::default_agent_md().as_ref(),
            );
        }
        // 安装预设 skills
        if let Err(e) = crate::assets::install_default_skills(&skill::skills_dir()) {
            crate::util::log::write_error_log(
                "[ChatApp::new]",
                &format!("安装预设 skills 失败: {}", e),
            );
        }
        // 安装预设 commands
        if let Err(e) = crate::assets::install_default_commands(&command::commands_dir()) {
            crate::util::log::write_error_log(
                "[ChatApp::new]",
                &format!("安装预设 commands 失败: {}", e),
            );
        }

        // 每次启动创建全新会话（session_id 由调用方生成）
        let mut model_list_state = ListState::default();
        if !agent_config.providers.is_empty() {
            model_list_state.select(Some(agent_config.active_index));
        }
        let theme = Theme::from_name(&agent_config.theme);
        let loaded_skills = skill::load_all_skills();
        let loaded_commands = command::load_all_commands();
        let (ask_req_tx, ask_req_rx) = mpsc::channel::<AskRequest>();
        let queued_tasks: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let pending_user_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let display_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let context_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let teammate_manager: Arc<Mutex<TeammateManager>> =
            Arc::new(Mutex::new(TeammateManager::new(
                Arc::clone(&pending_user_messages),
                Arc::clone(&display_messages),
                Arc::clone(&context_messages),
            )));
        let background_manager = Arc::new(BackgroundManager::new());
        let task_manager = Arc::new(TaskManager::new_with_session(&session_id));
        let hook_manager = Arc::new(Mutex::new(HookManager::load()));
        let invoked_skills = crate::command::chat::context::compact::new_invoked_skills_map();
        let mut tool_registry = ToolRegistry::new(ToolDefinitionParams {
            skills: loaded_skills.clone(),
            ask_tx: ask_req_tx,
            background_manager: Arc::clone(&background_manager),
            task_manager: Arc::clone(&task_manager),
            hook_manager: Arc::clone(&hook_manager),
            invoked_skills: Arc::clone(&invoked_skills),
            todos_file_path: crate::command::chat::storage::SessionPaths::new(&session_id)
                .todos_file(),
        });
        let todo_manager = Arc::clone(&tool_registry.todo_manager);

        // AgentTool 需要 provider 和 system_prompt 的共享引用（运行时动态获取）
        let default_provider = agent_config
            .providers
            .get(agent_config.active_index)
            .cloned()
            .unwrap_or_else(|| ModelProvider {
                name: String::new(),
                api_base: String::new(),
                api_key: String::new(),
                model: String::new(),
                supports_vision: false,
            });
        let agent_provider: Arc<Mutex<ModelProvider>> = Arc::new(Mutex::new(default_provider));
        let agent_system_prompt: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        // 注入 LLM provider 到 HookManager（LLM hook 执行时使用）
        if let Ok(mut mgr) = hook_manager.lock() {
            mgr.set_provider(Arc::clone(&agent_provider));
        }

        let disabled_tools_arc = Arc::new(agent_config.disabled_tools.clone());
        let deferred_tools_arc: Arc<Mutex<Vec<String>>> =
            Arc::new(Mutex::new(agent_config.deferred_tools.clone()));
        let session_loaded_deferred_arc: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // 子 agent 权限请求队列（TUI 和所有 agent 共享同一个 Arc）
        let permission_queue = Arc::new(PermissionQueue::new());
        // Plan 审批请求队列（TUI 和所有 teammate 共享同一个 Arc）
        let plan_approval_queue = Arc::new(PlanApprovalQueue::new());
        // 子 Agent 快照追踪器（/dump 从中读取正在运行的子 Agent）
        let sub_agent_tracker = Arc::new(SubAgentTracker::new());

        // 共享的 session id 槽：session 切换时 chat_app 会同步更新，teammate/subagent 据此定位 transcript
        let shared_session_id = Arc::new(Mutex::new(session_id.clone()));

        // 子 agent 上下文配置快照（send_message 时刷新）
        let agent_context_config = Arc::new(Mutex::new(
            crate::command::chat::tools::derived_shared::AgentContextConfig {
                max_history_messages: agent_config.max_history_messages,
                max_context_tokens: agent_config.max_context_tokens,
                compact: agent_config.compact.clone(),
            },
        ));
        // 子 agent 使用的 disabled_hooks 快照（Teammate 走 hook 链时用）
        let shared_disabled_hooks = Arc::new(Mutex::new(agent_config.disabled_hooks.clone()));

        // 构建 DerivedAgentShared（SubAgentTool / TeammateTool 共用）
        let shared_sub_agent_metrics = Arc::new(Mutex::new(
            crate::command::chat::tools::derived_shared::SubAgentMetrics::default(),
        ));
        let derived_agent_shared = DerivedAgentShared {
            background_manager: Arc::clone(&background_manager),
            provider: Arc::clone(&agent_provider),
            system_prompt: Arc::clone(&agent_system_prompt),
            jcli_config: Arc::new(JcliConfig::load()),
            hook_manager: Arc::clone(&hook_manager),
            task_manager: Arc::clone(&task_manager),
            disabled_tools: Arc::clone(&disabled_tools_arc),
            deferred_tools: Arc::clone(&deferred_tools_arc),
            session_loaded_deferred: Arc::clone(&session_loaded_deferred_arc),
            permission_queue: Arc::clone(&permission_queue),
            plan_approval_queue: Arc::clone(&plan_approval_queue),
            sub_agent_tracker: Arc::clone(&sub_agent_tracker),
            display_messages: Arc::clone(&display_messages),
            context_messages: Arc::clone(&context_messages),
            session_id: Arc::clone(&shared_session_id),
            plan_mode_state: Arc::clone(&tool_registry.plan_mode_state),
            agent_context_config: Arc::clone(&agent_context_config),
            disabled_hooks: Arc::clone(&shared_disabled_hooks),
            sub_agent_metrics: Arc::clone(&shared_sub_agent_metrics),
        };
        tool_registry.register(Box::new(
            crate::command::chat::tools::sub_agent::SubAgentTool {
                shared: derived_agent_shared.clone(),
            },
        ));
        tool_registry.register(Box::new(
            crate::command::chat::tools::teammate_tool::TeammateTool {
                shared: derived_agent_shared,
                teammate_manager: Arc::clone(&teammate_manager),
            },
        ));
        tool_registry.register(Box::new(
            crate::command::chat::tools::send_message::SendMessageTool {
                teammate_manager: Arc::clone(&teammate_manager),
            },
        ));
        tool_registry.register(Box::new(
            crate::command::chat::tools::ignore_message::IgnoreMessageTool {
                teammate_manager: Some(Arc::clone(&teammate_manager)),
            },
        ));
        // 注册 LoadTool，它持有 deferred_tools + session_loaded_deferred 的共享引用
        tool_registry.register(Box::new(
            crate::command::chat::tools::load_tool::LoadTool::new(
                Arc::clone(&deferred_tools_arc),
                Arc::clone(&session_loaded_deferred_arc),
            ),
        ));
        let tool_registry = Arc::new(tool_registry);
        let jcli_config = Arc::new(JcliConfig::load());

        // ── 注册内置 hook ──
        init_hooks::register_builtin_hooks(
            &hook_manager,
            &task_manager,
            &background_manager,
            &tool_registry,
            &teammate_manager,
            &todo_manager,
        );

        let new_app = Self {
            ui: UIState {
                input_buffer: TextBuffer::new(),
                mode: ChatMode::Chat,
                scroll_offset: usize::MAX,
                auto_scroll: true,
                browse_msg_index: 0,
                browse_scroll_offset: 0,
                browse_filter: String::new(),
                browse_role_filter: None,
                model_list_state,
                theme_list_state: ListState::default(),
                toast: None,
                msg_lines_cache: None,
                cached_mention_ranges: None,
                last_rendered_streaming_len: 0,
                last_stream_render_time: std::time::Instant::now(),
                config_provider_idx: 0,
                config_field_idx: 0,
                model_in_fields: false,
                model_field_idx: 0,
                config_editing: false,
                config_edit_buf: String::new(),
                config_edit_cursor: 0,
                theme,
                archives: Vec::new(),
                archive_list_index: 0,
                archive_default_name: String::new(),
                archive_custom_name: String::new(),
                archive_editing_name: false,
                archive_edit_cursor: 0,
                restore_confirm_needed: false,
                at_popup_active: false,
                at_popup_filter: String::new(),
                at_popup_start_pos: 0,
                at_popup_selected: 0,
                file_popup_active: false,
                file_popup_start_pos: 0,
                file_popup_filter: String::new(),
                file_popup_selected: 0,
                skill_popup_active: false,
                skill_popup_start_pos: 0,
                skill_popup_filter: String::new(),
                skill_popup_selected: 0,
                command_popup_active: false,
                command_popup_start_pos: 0,
                command_popup_filter: String::new(),
                command_popup_selected: 0,
                slash_popup_active: false,
                slash_popup_filter: String::new(),
                slash_popup_selected: 0,
                tool_interact_selected: 0,
                tool_interact_typing: false,
                tool_interact_input: String::new(),
                tool_interact_cursor: 0,
                tool_ask_mode: false,
                tool_ask_questions: Vec::new(),
                tool_ask_current_idx: 0,
                tool_ask_answers: Vec::new(),
                tool_ask_selections: Vec::new(),
                tool_ask_cursor: 0,
                tool_ask_drafts: Vec::new(),
                pending_system_prompt_edit: false,
                pending_agent_md_edit: false,
                pending_style_edit: false,
                image_cache: Arc::new(Mutex::new(ImageCache::new())),
                expand_tools: false,
                config_scroll_offset: 0,
                config_tab: ConfigTab::Model,
                session_list: Vec::new(),
                session_list_index: 0,
                session_restore_confirm: false,
                teammate_list_index: 0,
                quote_idx: {
                    let ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as usize;
                    ms % crate::command::chat::ui::quotes::quotes_count()
                },
                input_wrap_width: 0,
                pending_agent_perm: None,
                pending_plan_approval: None,
                compact_exempt_sublist: false,
                compact_exempt_idx: 0,
                tools_in_options: false,
                tools_option_idx: 0,
                auto_approve: false,
                commands_mode: CommandsMode::Normal,
                commands_source_idx: 0,
                pending_command_create: false,
                command_create_source: CommandSource::User,
                mouse_selection: None,
                msg_area_inner: None,
                context_menu: None,
                help_lines_cache: None,
                help_area_inner: None,
                help_scroll_offset: 0,
                config_list_area: None,
                config_provider_area: None,
                config_provider_lines: Vec::new(),
                config_tab_bar_y: None,
                config_field_lines: Vec::new(),
                config_tab_hitboxes: Vec::new(),
                config_lines_cache: None,
                config_content_inner: None,
                config_content_scroll: 0,
            },
            state: ChatState {
                agent_config,
                streaming_content: Arc::new(Mutex::new(String::new())),
                streaming_reasoning_content: Arc::new(Mutex::new(String::new())),
                is_loading: false,
                loaded_skills,
                loaded_commands,
                queued_tasks,
                pending_user_messages: Arc::clone(&pending_user_messages),
                retry_hint: None,
            },
            tool_executor: ToolExecutor::new(),
            main_agent: None,
            tool_registry,
            jcli_config,
            background_manager,
            task_manager,
            todo_manager,
            ask_response_tx: None,
            ask_request_rx: Some(ask_req_rx),
            hook_manager: Arc::clone(&hook_manager),
            sandbox: Sandbox::new(),
            session_id,
            shared_session_id,
            persisted_message_count: 0,
            persisted_display_count: 0,
            ws_bridge: None,
            remote_connected: false,
            derived_agent_provider: agent_provider,
            derived_agent_system_prompt: agent_system_prompt,
            derived_agent_context_config: agent_context_config,
            derived_agent_disabled_hooks: shared_disabled_hooks,
            display_messages,
            display_read_offset: 0,
            context_messages,
            context_read_offset: 0,
            context_tokens: Arc::new(Mutex::new(0)),
            teammate_manager,
            sub_agent_tracker,
            permission_queue,
            plan_approval_queue,
            sub_agent_metrics: shared_sub_agent_metrics,
            deferred_tools: deferred_tools_arc,
            session_loaded_deferred: session_loaded_deferred_arc,
            invoked_skills,
            file_index: FileIndex::new(),
        };

        // 执行 SessionStart hook（fire-and-forget，不阻塞启动）
        {
            let should_fire = new_app
                .hook_manager
                .lock()
                .map(|m| m.has_hooks_for(HookEvent::SessionStart))
                .unwrap_or(false);
            if should_fire {
                let ctx = HookContext {
                    event: HookEvent::SessionStart,
                    messages: Some(
                        safe_lock(&new_app.context_messages, "SessionStart::ctx_msgs").clone(),
                    ),
                    session_id: Some(new_app.session_id.clone()),
                    cwd: std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string()),
                    ..Default::default()
                };
                HookManager::execute_fire_and_forget(
                    Arc::clone(&new_app.hook_manager),
                    HookEvent::SessionStart,
                    ctx,
                    new_app.state.agent_config.disabled_hooks.clone(),
                );
            }
        }

        new_app
    }

    /// 切换到下一个主题
    pub fn switch_theme(&mut self) {
        self.state.agent_config.theme = self.state.agent_config.theme.next();
        self.ui.theme = Theme::from_name(&self.state.agent_config.theme);
        self.ui.msg_lines_cache = None;
    }

    /// 显示一条 toast 提示消息（is_error=true 时以错误色显示）
    pub fn show_toast(&mut self, msg: impl Into<String>, is_error: bool) {
        self.ui.toast = Some((msg.into(), is_error, std::time::Instant::now()));
    }

    /// 清理过期的 toast
    pub fn tick_toast(&mut self) {
        if let Some((_, _, created)) = &self.ui.toast
            && created.elapsed().as_secs() >= TOAST_DURATION_SECS
        {
            self.ui.toast = None;
        }
    }

    /// 向上滚动消息
    pub fn scroll_up(&mut self) {
        self.ui.scroll_offset = self.ui.scroll_offset.saturating_sub(3);
        self.ui.auto_scroll = false;
    }

    /// 向下滚动消息
    pub fn scroll_down(&mut self) {
        self.ui.scroll_offset = self.ui.scroll_offset.saturating_add(3);
    }
}

#[cfg(test)]
mod tests {
    use super::ChatApp;
    use crate::command::chat::app::types::{ToolCallStatus, ToolExecStatus};
    use crate::command::chat::app::ui_state::ChatMode;

    #[test]
    fn reject_with_reason_resets_tool_confirm_ui_for_next_pending_tool() {
        let mut app = ChatApp::new("test-reject-next-tool".to_string());
        app.ui.mode = ChatMode::ToolConfirm;
        app.ui.tool_interact_selected = 3;
        app.ui.tool_interact_typing = true;
        app.ui.tool_interact_input = "不要执行".to_string();
        app.ui.tool_interact_cursor = app.ui.tool_interact_input.chars().count();
        app.tool_executor.active_tool_calls = vec![
            ToolCallStatus {
                tool_call_id: "tool-1".to_string(),
                tool_name: "Write".to_string(),
                arguments: "{}".to_string(),
                confirm_message: "first".to_string(),
                status: ToolExecStatus::PendingConfirm,
                tool_description: None,
            },
            ToolCallStatus {
                tool_call_id: "tool-2".to_string(),
                tool_name: "Edit".to_string(),
                arguments: "{}".to_string(),
                confirm_message: "second".to_string(),
                status: ToolExecStatus::PendingConfirm,
                tool_description: None,
            },
        ];

        app.reject_pending_tool("不要执行");

        assert!(matches!(app.ui.mode, ChatMode::ToolConfirm));
        assert_eq!(app.tool_executor.pending_tool_idx, 1);
        assert_eq!(app.ui.tool_interact_selected, 0);
        assert!(!app.ui.tool_interact_typing);
        assert!(app.ui.tool_interact_input.is_empty());
        assert_eq!(app.ui.tool_interact_cursor, 0);
    }

    #[test]
    fn reject_last_pending_tool_exits_tool_confirm_mode() {
        let mut app = ChatApp::new("test-reject-last-tool".to_string());
        app.ui.mode = ChatMode::ToolConfirm;
        app.ui.tool_interact_selected = 3;
        app.ui.tool_interact_typing = true;
        app.ui.tool_interact_input = "不要执行".to_string();
        app.ui.tool_interact_cursor = app.ui.tool_interact_input.chars().count();
        app.tool_executor.active_tool_calls = vec![ToolCallStatus {
            tool_call_id: "tool-1".to_string(),
            tool_name: "Write".to_string(),
            arguments: "{}".to_string(),
            confirm_message: "only".to_string(),
            status: ToolExecStatus::PendingConfirm,
            tool_description: None,
        }];

        app.reject_pending_tool("不要执行");

        assert!(matches!(app.ui.mode, ChatMode::Chat));
    }
}
