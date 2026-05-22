use super::ChatApp;
use crate::command::chat::remote::protocol::WsOutbound;
use crate::util::safe_lock;

impl ChatApp {
    /// 广播 WebSocket 消息给远程客户端
    pub fn broadcast_ws(&self, msg: WsOutbound) {
        if let Some(ref ws) = self.ws_bridge {
            ws.broadcast(msg);
        }
    }

    /// 构建全量同步消息（复用于 Sync / SwitchSession / NewSession）
    pub fn build_sync_outbound(&self) -> WsOutbound {
        use crate::command::chat::remote::protocol::{SyncMessage, SyncToolCall};
        let messages: Vec<SyncMessage> = safe_lock(&self.context_messages, "build_sync_outbound")
            .iter()
            .map(|m| SyncMessage {
                role: m.role.to_string(),
                content: m.content.clone(),
                tool_calls: m.tool_calls.as_ref().map(|tc| {
                    tc.iter()
                        .map(|t| SyncToolCall {
                            id: t.id.clone(),
                            name: t.name.clone(),
                            arguments: t.arguments.clone(),
                        })
                        .collect()
                }),
                tool_call_id: m.tool_call_id.clone(),
            })
            .collect();
        let status = if self.state.is_loading {
            "loading"
        } else if self.ui.mode == crate::command::chat::app::ui_state::ChatMode::ToolConfirm {
            "tool_confirm"
        } else {
            "idle"
        };
        let model = self.active_model_name().to_string();
        let context_tokens = *safe_lock(&self.context_tokens, "build_sync_outbound::ctx_tokens");
        let message_count =
            safe_lock(&self.context_messages, "build_sync_outbound::msg_count").len();
        WsOutbound::SessionSync {
            messages,
            status: status.to_string(),
            model,
            context_tokens,
            message_count,
            auto_approve: self.ui.auto_approve,
        }
    }

    /// 广播配置数据到远程客户端
    pub fn broadcast_config_state(&mut self) {
        use crate::command::chat::app::ui_state::ConfigTab;
        use crate::command::chat::remote::protocol::{ConfigField, ModelInfo, ThemeInfo};

        let tab = match self.ui.config_tab {
            ConfigTab::Model => "model",
            ConfigTab::Session => "session",
            ConfigTab::Global => "global",
            ConfigTab::Tools => "tools",
            ConfigTab::Skills => "skills",
            ConfigTab::Hooks => "hooks",
            ConfigTab::Commands => "commands",
            ConfigTab::Teammates => "teammates",
            ConfigTab::Archive => "archive",
        };

        let fields = match self.ui.config_tab {
            ConfigTab::Model => {
                let mut fields = Vec::new();
                for (i, p) in self.state.agent_config.providers.iter().enumerate() {
                    let is_active = i == self.state.agent_config.active_index;
                    fields.push(ConfigField {
                        key: format!("provider_{}", i),
                        label: p.name.clone(),
                        value: format!("{} @ {}", p.model, p.api_base),
                        field_type: "select".to_string(),
                        editable: false,
                        options: None,
                    });
                    if is_active {
                        fields.push(ConfigField {
                            key: "active_provider".into(),
                            label: "当前模型".into(),
                            value: p.name.clone(),
                            field_type: "text".into(),
                            editable: false,
                            options: None,
                        });
                    }
                }
                // 也发送模型列表供快速切换
                let models: Vec<ModelInfo> = self
                    .state
                    .agent_config
                    .providers
                    .iter()
                    .map(|p| ModelInfo {
                        name: p.name.clone(),
                        model: p.model.clone(),
                        provider: p.api_base.clone(),
                        supports_vision: p.supports_vision,
                    })
                    .collect();
                self.broadcast_ws(WsOutbound::ModelList {
                    models,
                    active_index: self.state.agent_config.active_index,
                });
                // 同时发送主题列表供远程快速切换
                {
                    use crate::theme::ThemeName;
                    let all_themes = ThemeName::all();
                    let themes: Vec<ThemeInfo> = all_themes
                        .iter()
                        .map(|t| ThemeInfo {
                            name: t.to_str().to_string(),
                            display_name: t.display_name().to_string(),
                        })
                        .collect();
                    let active_idx = all_themes
                        .iter()
                        .position(|n| *n == self.state.agent_config.theme)
                        .unwrap_or(0);
                    self.broadcast_ws(WsOutbound::ThemeList {
                        themes,
                        active_index: active_idx,
                    });
                }
                fields
            }
            ConfigTab::Global => {
                let cfg = &self.state.agent_config;
                vec![
                    ConfigField {
                        key: "max_history_messages".into(),
                        label: "最大历史消息数".into(),
                        value: cfg.max_history_messages.to_string(),
                        field_type: "text".into(),
                        editable: true,
                        options: None,
                    },
                    ConfigField {
                        key: "max_context_tokens".into(),
                        label: "最大上下文 Token".into(),
                        value: cfg.max_context_tokens.to_string(),
                        field_type: "text".into(),
                        editable: true,
                        options: None,
                    },
                    ConfigField {
                        key: "max_tool_rounds".into(),
                        label: "最大工具轮数".into(),
                        value: cfg.max_tool_rounds.to_string(),
                        field_type: "text".into(),
                        editable: true,
                        options: None,
                    },
                    ConfigField {
                        key: "tools_enabled".into(),
                        label: "启用工具".into(),
                        value: cfg.tools_enabled.to_string(),
                        field_type: "bool".into(),
                        editable: true,
                        options: None,
                    },
                    ConfigField {
                        key: "tool_confirm_timeout".into(),
                        label: "工具确认超时(秒)".into(),
                        value: cfg.tool_confirm_timeout.to_string(),
                        field_type: "text".into(),
                        editable: true,
                        options: None,
                    },
                ]
            }
            ConfigTab::Tools => {
                let all_tools: Vec<String> = self
                    .tool_registry
                    .tool_names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();
                let disabled = &self.state.agent_config.disabled_tools;
                all_tools
                    .into_iter()
                    .map(|name| ConfigField {
                        key: name.clone(),
                        label: name.clone(),
                        value: (!disabled.contains(&name)).to_string(),
                        field_type: "bool".into(),
                        editable: true,
                        options: None,
                    })
                    .collect()
            }
            ConfigTab::Skills => {
                let skills = &self.state.loaded_skills;
                let disabled = &self.state.agent_config.disabled_skills;
                skills
                    .iter()
                    .map(|s| {
                        let name = s.frontmatter.name.clone();
                        ConfigField {
                            key: name.clone(),
                            label: name.clone(),
                            value: (!disabled.contains(&name)).to_string(),
                            field_type: "bool".into(),
                            editable: true,
                            options: None,
                        }
                    })
                    .collect()
            }
            ConfigTab::Session
            | ConfigTab::Archive
            | ConfigTab::Hooks
            | ConfigTab::Commands
            | ConfigTab::Teammates => vec![],
        };

        self.broadcast_ws(WsOutbound::ConfigData {
            tab: tab.to_string(),
            fields,
        });
    }

    /// 广播归档确认状态到远程客户端
    pub fn broadcast_archive_confirm_state(&self) {
        // ArchiveConfirm 状态已通过 session_sync 的 status 字段表达
        // 这里额外广播默认归档名
        self.broadcast_ws(WsOutbound::Status {
            state: "archive_confirm".to_string(),
        });
    }

    /// 广播归档列表到远程客户端
    pub fn broadcast_archive_list_state(&self) {
        use crate::command::chat::remote::protocol::ArchiveInfo;
        let archives: Vec<ArchiveInfo> = self
            .ui
            .archives
            .iter()
            .map(|a| ArchiveInfo {
                name: a.name.clone(),
                created_at: a.created_at.clone(),
                message_count: a.messages.len(),
            })
            .collect();
        self.broadcast_ws(WsOutbound::ArchiveList { archives });
    }

    /// 广播会话列表状态到远程客户端
    pub fn broadcast_session_list_state(&self) {
        let sessions = crate::command::chat::storage::list_sessions();
        self.broadcast_ws(WsOutbound::SessionList { sessions });
    }

    /// 从远程客户端注入一条消息（模拟用户输入并发送）
    /// 注意：不广播 user message 回去，发送方 Web 端已经本地显示了
    ///
    /// 如果当前正在 loading（agent loop 运行中），消息追加到待处理队列，
    /// 与 TUI 本地模式下 Enter 的行为一致。
    pub fn inject_remote_message(&mut self, content: &str) {
        use crate::command::chat::infra::command;
        use crate::command::chat::storage::{ChatMessage, MessageRole};

        let text = content.trim().to_string();
        if text.is_empty() {
            return;
        }

        // 展开 @command:name 引用
        let text = command::expand_command_mentions(
            &text,
            &self.state.loaded_commands,
            &self.state.agent_config.disabled_commands,
        );

        if self.state.is_loading {
            // agent loop 运行中：追加到 pending 队列 + 双通道，下一轮 loop 会处理
            let user_msg = ChatMessage::text(MessageRole::User, &text);
            self.push_both_channels(user_msg);
            {
                let mut pending = crate::util::safe_lock(
                    &self.state.pending_user_messages,
                    "inject_remote_message::pending",
                );
                pending.push(ChatMessage::text(MessageRole::User, &text));
            }
            self.ui.msg_lines_cache = None;
            self.ui.auto_scroll = true;
            self.ui.scroll_offset = usize::MAX;
        } else {
            self.send_message_internal(text);
        }
    }
}
