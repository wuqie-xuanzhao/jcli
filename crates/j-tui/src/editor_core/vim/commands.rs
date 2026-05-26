//! 命令面板类型与辅助函数

/// 命令面板项
#[derive(Debug, Clone)]
pub struct CmdItem {
    pub name: &'static str,
    pub desc: &'static str,
}

/// 命令面板所有可用命令
pub const COMMANDS: &[CmdItem] = &[
    CmdItem {
        name: "save",
        desc: "保存/提交",
    },
    CmdItem {
        name: "quit",
        desc: "取消退出",
    },
    CmdItem {
        name: "search",
        desc: "搜索",
    },
    CmdItem {
        name: "wrap",
        desc: "开启折行",
    },
    CmdItem {
        name: "nowrap",
        desc: "关闭折行",
    },
    CmdItem {
        name: "jump",
        desc: "跳转到指定行 (如 /jump 10)",
    },
    CmdItem {
        name: "undo",
        desc: "撤销",
    },
    CmdItem {
        name: "redo",
        desc: "重做",
    },
    CmdItem {
        name: "tohead",
        desc: "跳到文件开头",
    },
    CmdItem {
        name: "toend",
        desc: "跳到文件末尾",
    },
    CmdItem {
        name: "theme",
        desc: "切换主题",
    },
    CmdItem {
        name: "line-number",
        desc: "显示行号",
    },
    CmdItem {
        name: "no-line-number",
        desc: "隐藏行号",
    },
    CmdItem {
        name: "help",
        desc: "显示帮助指南",
    },
];

/// Insert 模式下输入 `/` 触发的命令面板项
///
/// 与 Normal 模式 COMMANDS 不同，这些命令的语义是「向文档插入内容」，
/// 而不是「执行编辑器动作」。
pub const INSERT_COMMANDS: &[CmdItem] = &[
    CmdItem {
        name: "image",
        desc: "插入图片 ![]()",
    },
    CmdItem {
        name: "/",
        desc: "输入 / 字符",
    },
];

/// 根据筛选文本过滤命令列表
pub fn filter_commands(filter: &str) -> Vec<&'static CmdItem> {
    if filter.is_empty() {
        COMMANDS.iter().collect()
    } else {
        let filter_lower = filter.to_lowercase();
        COMMANDS
            .iter()
            .filter(|cmd| {
                cmd.name.contains(&filter_lower)
                    || cmd.name.starts_with(&filter_lower)
                    || cmd.desc.contains(&filter_lower)
            })
            .collect()
    }
}

/// 根据筛选文本过滤 Insert 模式命令列表
///
/// 与 `filter_commands` 不同：仅按 `name` 前缀匹配。这样输入 `https` 这类
/// 真实文本时面板会立即变空（editor 据此自动关闭面板），不会因为 desc 里
/// 含某个字而误留弹窗。
pub fn filter_insert_commands(filter: &str) -> Vec<&'static CmdItem> {
    if filter.is_empty() {
        return INSERT_COMMANDS.iter().collect();
    }
    let filter_lower = filter.to_lowercase();
    INSERT_COMMANDS
        .iter()
        .filter(|cmd| cmd.name.to_lowercase().starts_with(&filter_lower))
        .collect()
}

/// 解析命令面板输入，提取命令名和参数
pub fn parse_command(input: &str) -> (&str, &str) {
    if let Some(space_pos) = input.find(' ') {
        (&input[..space_pos], input[space_pos + 1..].trim())
    } else {
        (input, "")
    }
}
