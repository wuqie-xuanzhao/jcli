//! 补全规则定义：参数类型枚举与命令参数补全规则表

use crate::constants::{
    ALL_SECTIONS, LIST_ALL, cmd, config_key, rmeta_action, search_flag, time_function,
};

/// 命令参数补全提示类型
#[derive(Clone)]
#[allow(dead_code)]
pub enum ArgHint {
    Alias,
    Category,
    Section,
    SectionKeys(String),
    Fixed(Vec<&'static str>),
    /// Flag 补全：当前词以 `-` 开头时触发，可出现在任意参数位置
    Flags(Vec<&'static str>),
    /// 动态补全：根据前面第 N 个 positional 参数作为 section 名，补全该 section 下的 key
    DynamicSectionKeys {
        section_arg_index: usize,
    },
    Placeholder(&'static str),
    FilePath,
    None,
}

/// 获取所有命令的参数补全规则定义
pub fn command_completion_rules() -> Vec<(&'static [&'static str], Vec<ArgHint>)> {
    vec![
        (
            cmd::SET,
            vec![ArgHint::Placeholder("<alias>"), ArgHint::FilePath],
        ),
        (cmd::REMOVE, vec![ArgHint::Alias]),
        (
            cmd::RENAME,
            vec![ArgHint::Alias, ArgHint::Placeholder("<new_alias>")],
        ),
        (cmd::MODIFY, vec![ArgHint::Alias, ArgHint::FilePath]),
        (cmd::TAG, vec![ArgHint::Alias, ArgHint::Category]),
        (cmd::UNTAG, vec![ArgHint::Alias, ArgHint::Category]),
        (
            cmd::LIST,
            vec![ArgHint::Fixed({
                let mut v: Vec<&'static str> = vec!["", LIST_ALL];
                for s in ALL_SECTIONS {
                    v.push(s);
                }
                v
            })],
        ),
        (
            cmd::CONTAIN,
            vec![ArgHint::Alias, ArgHint::Placeholder("<sections>")],
        ),
        (
            cmd::LOG,
            vec![
                ArgHint::Fixed(vec![config_key::MODE]),
                ArgHint::Fixed(vec![config_key::VERBOSE, config_key::CONCISE]),
            ],
        ),
        (
            cmd::CONFIG,
            vec![
                ArgHint::Section,
                ArgHint::DynamicSectionKeys {
                    section_arg_index: 0,
                },
                ArgHint::Placeholder("<value>"),
            ],
        ),
        (cmd::REPORT, vec![ArgHint::Placeholder("<content>")]),
        (
            cmd::REPORTCTL,
            vec![
                ArgHint::Fixed(vec![
                    rmeta_action::NEW,
                    rmeta_action::SYNC,
                    rmeta_action::PUSH,
                    rmeta_action::PULL,
                    rmeta_action::SET_URL,
                    rmeta_action::OPEN,
                ]),
                ArgHint::Placeholder("<date|message|url>"),
            ],
        ),
        (
            cmd::CHECK,
            vec![ArgHint::Fixed(vec!["open", "<line_count>"])],
        ),
        (
            cmd::SEARCH,
            vec![
                ArgHint::Placeholder("<line_count|all>"),
                ArgHint::Placeholder("<target>"),
                ArgHint::Fixed(vec![search_flag::FUZZY_SHORT, search_flag::FUZZY]),
            ],
        ),
        (
            cmd::TODO,
            vec![
                ArgHint::Fixed(vec!["list", "add"]),
                ArgHint::Placeholder("<content>"),
            ],
        ),
        (
            cmd::CHAT,
            vec![
                ArgHint::Flags(vec!["--continue", "-c", "--session", "--remote"]),
                ArgHint::Placeholder("<message>"),
            ],
        ),
        (
            cmd::SCRIPT,
            vec![
                ArgHint::Placeholder("<script_name>"),
                ArgHint::Placeholder("<script_content>"),
            ],
        ),
        (
            cmd::TIME,
            vec![
                ArgHint::Fixed(vec![time_function::COUNTDOWN]),
                ArgHint::Placeholder("<duration>"),
            ],
        ),
        (cmd::COMPLETION, vec![ArgHint::Fixed(vec!["zsh", "bash"])]),
        (cmd::VERSION, vec![]),
        (cmd::HELP, vec![]),
        (cmd::CLEAR, vec![]),
        (cmd::EXIT, vec![]),
        (cmd::UPDATE, vec![ArgHint::Fixed(vec!["--check"])]),
        (cmd::MD, vec![ArgHint::FilePath]),
    ]
}
