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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SourceRange {
    pub start_line: usize,
    #[allow(dead_code)]
    pub end_line: usize,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct ParsedDocument {
    pub blocks: Vec<Block>,
    #[allow(dead_code)]
    pub line_to_block: Vec<Option<usize>>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Block {
    #[allow(dead_code)]
    pub source: SourceRange,
    pub kind: BlockKind,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum BlockKind {
    Paragraph(Vec<Inline>),
    Heading { level: u8, content: Vec<Inline> },
    CodeBlock { lang: String, code: String },
    Table(TableData),
    List(ListData),
    BlockQuote(Vec<Block>),
    Rule,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TableData {
    pub alignments: Vec<Alignment>,
    pub rows: Vec<Vec<Vec<Inline>>>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ListData {
    pub ordered: bool,
    pub start_index: Option<u64>,
    pub items: Vec<ListItem>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub content: Vec<Inline>,
    pub children: Vec<Block>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    Link { text: Vec<Inline>, url: String },
    SoftBreak,
    HardBreak,
}
