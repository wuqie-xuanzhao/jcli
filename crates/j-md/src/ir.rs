//! Markdown 中间表示（IR）类型定义
//!
//! Parser 输出 `ParsedDocument`，消费者基于 IR 生成各自平台的渲染输出。
//! IR 与终端宽度、主题、平台无关，可被多次渲染。

/// 表格列对齐方式
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    None,
    Left,
    Center,
    Right,
}

/// 源码位置范围（用于调试和错误定位）
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SourceRange {
    /// 起始行号（0-indexed）
    pub start_line: usize,
    #[allow(dead_code)]
    /// 结束行号（0-indexed）
    pub end_line: usize,
}

/// 解析后的 Markdown 文档结构
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct ParsedDocument {
    /// 文档中的所有块级元素
    pub blocks: Vec<Block>,
    #[allow(dead_code)]
    /// 行号到块索引的映射（用于快速定位）
    pub line_to_block: Vec<Option<usize>>,
}

/// 单个块级元素及其源码位置
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Block {
    #[allow(dead_code)]
    /// 源码位置范围
    pub source: SourceRange,
    /// 块内容类型
    pub kind: BlockKind,
}

/// 块级元素类型
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum BlockKind {
    /// 普通段落
    Paragraph(Vec<Inline>),
    /// 标题（level: 1-6）
    Heading { level: u8, content: Vec<Inline> },
    /// 代码块（含语言标识）
    CodeBlock { lang: String, code: String },
    /// 表格
    Table(TableData),
    /// 列表（有序或无序）
    List(ListData),
    /// 引用块
    BlockQuote(Vec<Block>),
    /// 水平分隔线
    Rule,
}

/// 表格数据结构
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TableData {
    /// 各列对齐方式
    pub alignments: Vec<Alignment>,
    /// 表格行（每行为单元格列表，每个单元格为 Inline 列表）
    pub rows: Vec<Vec<Vec<Inline>>>,
}

/// 列表数据结构
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ListData {
    /// 是否为有序列表
    pub ordered: bool,
    /// 有序列表起始序号（仅对有序列表有效）
    pub start_index: Option<u64>,
    /// 列表项
    pub items: Vec<ListItem>,
}

/// 单个列表项
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ListItem {
    /// 任务列表勾选状态（None 表示非任务列表项）
    pub checked: Option<bool>,
    /// 列表项内容
    pub content: Vec<Inline>,
    /// 子列表（嵌套列表）
    pub children: Vec<Block>,
}

/// 行内元素类型
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum Inline {
    /// 普通文本
    Text(String),
    /// 加粗文本
    Strong(Vec<Inline>),
    /// 斜体文本
    Emphasis(Vec<Inline>),
    /// 删除线文本
    Strikethrough(Vec<Inline>),
    /// 行内代码
    Code(String),
    /// 链接
    Link { text: Vec<Inline>, url: String },
    /// 软换行（渲染时转为空格）
    SoftBreak,
    /// 硬换行（渲染时保留换行）
    HardBreak,
}
