pub mod alias;
pub mod category;
pub mod chat;
pub mod handler;
pub mod help;
pub mod list;
pub mod lock;
pub mod notebook;
pub mod open;
pub mod read;
pub mod report;
pub mod script;
pub mod system;
pub mod time;
pub mod todo;
pub mod update;

use crate::cli::SubCmd;
use crate::config::YamlConfig;
use crate::constants;

/// 所有内置命令的关键字列表（用于判断别名冲突）
/// 统一从 constants::cmd 模块获取，避免多处重复定义
pub fn all_command_keywords() -> Vec<&'static str> {
    constants::cmd::all_keywords()
}

/// 命令分发执行
pub fn dispatch(subcmd: SubCmd, config: &mut YamlConfig) {
    subcmd.into_handler().execute(config);
}

#[cfg(test)]
mod tests {
    use super::all_command_keywords;
    use crate::constants::cmd;
    use std::collections::HashSet;

    #[test]
    fn all_keywords_includes_every_group() {
        let keywords = all_command_keywords();
        let set: HashSet<&str> = keywords.iter().copied().collect();

        // Each group's primary name must be present
        for (group_name, group_slice) in [
            ("set", cmd::SET),
            ("remove", cmd::REMOVE),
            ("rename", cmd::RENAME),
            ("modify", cmd::MODIFY),
            ("tag", cmd::TAG),
            ("untag", cmd::UNTAG),
            ("list", cmd::LIST),
            ("contain", cmd::CONTAIN),
            ("report", cmd::REPORT),
            ("reportctl", cmd::REPORTCTL),
            ("check", cmd::CHECK),
            ("search", cmd::SEARCH),
            ("todo", cmd::TODO),
            ("chat", cmd::CHAT),
            ("script", cmd::SCRIPT),
            ("time", cmd::TIME),
            ("log", cmd::LOG),
            ("config", cmd::CONFIG),
            ("clear", cmd::CLEAR),
            ("version", cmd::VERSION),
            ("help", cmd::HELP),
            ("exit", cmd::EXIT),
            ("completion", cmd::COMPLETION),
            ("update", cmd::UPDATE),
            ("md", cmd::MD),
            ("notebook", cmd::NOTEBOOK),
            ("lock", cmd::LOCK),
            ("unlock", cmd::UNLOCK),
            ("read", cmd::READ),
        ] {
            for kw in group_slice {
                assert!(
                    set.contains(kw),
                    "group '{group_name}' keyword '{kw}' missing from all_keywords()"
                );
            }
        }
    }

    #[test]
    fn all_keywords_no_duplicates() {
        let keywords = all_command_keywords();
        let set: HashSet<&str> = keywords.iter().copied().collect();
        assert_eq!(
            keywords.len(),
            set.len(),
            "all_command_keywords() contains duplicate entries"
        );
    }

    #[test]
    fn all_keywords_known_count() {
        let keywords = all_command_keywords();
        // Expected count: sum of all group slices
        let mut expected = 0;
        let groups: &[&[&str]] = &[
            cmd::SET,
            cmd::REMOVE,
            cmd::RENAME,
            cmd::MODIFY,
            cmd::TAG,
            cmd::UNTAG,
            cmd::LIST,
            cmd::CONTAIN,
            cmd::REPORT,
            cmd::REPORTCTL,
            cmd::CHECK,
            cmd::SEARCH,
            cmd::TODO,
            cmd::CHAT,
            cmd::SCRIPT,
            cmd::TIME,
            cmd::LOG,
            cmd::CONFIG,
            cmd::CLEAR,
            cmd::VERSION,
            cmd::HELP,
            cmd::EXIT,
            cmd::COMPLETION,
            cmd::AGENT,
            cmd::SYSTEM,
            cmd::UPDATE,
            cmd::MD,
            cmd::NOTEBOOK,
            cmd::LOCK,
            cmd::UNLOCK,
            cmd::READ,
        ];
        for g in groups {
            expected += g.len();
        }
        assert_eq!(
            keywords.len(),
            expected,
            "all_command_keywords() count changed; if intentional, update this test"
        );
    }
}
