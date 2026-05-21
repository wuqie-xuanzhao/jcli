mod table;

use crate::ir::{
    Alignment, Block, BlockKind, Inline, ListData, ListItem, ParsedDocument, SourceRange,
    TableData,
};
use pulldown_cmark::{Event, Tag, TagEnd};

// ---------------------------------------------------------------------------
// ParseContext — IR accumulation state for the markdown event loop
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum InlineContainer {
    Strong,
    Emphasis,
    Strikethrough,
    Link { url: String },
}

#[derive(Debug)]
struct ListFrame {
    ordered: bool,
    start_index: Option<u64>,
    items: Vec<ListItem>,
}

#[derive(Debug, Default)]
struct ItemFrame {
    checked: Option<bool>,
    content: Vec<Inline>,
    children: Vec<Block>,
}

struct ParseContext {
    blocks: Vec<Block>,
    current_source: SourceRange,
    current_inlines: Vec<Inline>,
    inline_stack: Vec<InlineContainer>,
    inline_children_stack: Vec<Vec<Inline>>,
    in_code_block: bool,
    code_block_content: String,
    code_block_lang: String,
    list_stack: Vec<ListFrame>,
    item_stack: Vec<ItemFrame>,
    heading_level: Option<u8>,
    blockquote_stack: Vec<Vec<Block>>,
    image_url: Option<String>,
    image_alt: String,
    in_table: bool,
    table_rows: Vec<Vec<Vec<Inline>>>,
    current_row: Vec<Vec<Inline>>,
    current_cell_inlines: Vec<Inline>,
    table_alignments: Vec<Alignment>,
    table_inline_stack: Vec<InlineContainer>,
    table_inline_children_stack: Vec<Vec<Inline>>,
}

impl ParseContext {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            current_source: SourceRange::default(),
            current_inlines: Vec::new(),
            inline_stack: Vec::new(),
            inline_children_stack: Vec::new(),
            in_code_block: false,
            code_block_content: String::new(),
            code_block_lang: String::new(),
            list_stack: Vec::new(),
            item_stack: Vec::new(),
            heading_level: None,
            blockquote_stack: Vec::new(),
            image_url: None,
            image_alt: String::new(),
            in_table: false,
            table_rows: Vec::new(),
            current_row: Vec::new(),
            current_cell_inlines: Vec::new(),
            table_alignments: Vec::new(),
            table_inline_stack: Vec::new(),
            table_inline_children_stack: Vec::new(),
        }
    }

    fn current_inline_target(&mut self) -> &mut Vec<Inline> {
        if self.in_table {
            return self.table_inline_target();
        }
        if let Some(children) = self.inline_children_stack.last_mut() {
            return children;
        }
        if let Some(item) = self.item_stack.last_mut() {
            return &mut item.content;
        }
        &mut self.current_inlines
    }

    fn table_inline_target(&mut self) -> &mut Vec<Inline> {
        if let Some(children) = self.table_inline_children_stack.last_mut() {
            children
        } else {
            &mut self.current_cell_inlines
        }
    }

    fn flush_paragraph(&mut self) {
        if self.current_inlines.is_empty() {
            return;
        }
        if !self.item_stack.is_empty() {
            return;
        }
        let inlines = std::mem::take(&mut self.current_inlines);
        let block = Block {
            source: self.current_source,
            kind: BlockKind::Paragraph(inlines),
        };
        self.push_block(block);
    }

    fn push_block(&mut self, block: Block) {
        if let Some(item) = self.item_stack.last_mut() {
            item.children.push(block);
            return;
        }
        if let Some(bq_blocks) = self.blockquote_stack.last_mut() {
            bq_blocks.push(block);
        } else {
            self.blocks.push(block);
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        self.current_inline_target().push(inline);
    }
}

// ---------------------------------------------------------------------------
// Table separator normalization
// ---------------------------------------------------------------------------

fn needs_table_separator_fix(md: &str) -> bool {
    let lines: Vec<&str> = md.lines().collect();
    for i in 1..lines.len() {
        let prev = lines[i - 1].trim();
        let curr = lines[i].trim();
        if prev.starts_with('|') && is_separator_row(curr) {
            let header_cols = count_pipe_cells(prev);
            let sep_cols = count_pipe_cells(curr);
            if header_cols > 1 && sep_cols < header_cols {
                return true;
            }
        }
    }
    false
}

fn normalize_table_separators(md: &str) -> String {
    let lines: Vec<&str> = md.lines().collect();
    let mut result = String::with_capacity(md.len());
    let mut modified = false;

    for i in 0..lines.len() {
        if i > 0 {
            let prev = lines[i - 1].trim();
            let curr = lines[i].trim();
            if prev.starts_with('|') && is_separator_row(curr) {
                let header_cols = count_pipe_cells(prev);
                let sep_cols = count_pipe_cells(curr);
                if header_cols > 1 && sep_cols < header_cols {
                    let mut fixed = String::from("|");
                    for _ in 0..header_cols {
                        fixed.push_str("---|");
                    }
                    result.push_str(&fixed);
                    modified = true;
                    result.push('\n');
                    continue;
                }
            }
        }
        result.push_str(lines[i]);
        result.push('\n');
    }

    if modified {
        if md.ends_with('\n') && !result.ends_with('\n') {
            result.push('\n');
        } else if !md.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }
        result
    } else {
        md.to_string()
    }
}

fn is_separator_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return false;
    }
    let inner = trimmed.trim_matches('|').trim();
    !inner.is_empty()
        && inner
            .chars()
            .all(|c| c == '-' || c == ':' || c == ' ' || c == '|')
}

fn count_pipe_cells(line: &str) -> usize {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return 0;
    }
    let segments: Vec<&str> = trimmed.split('|').collect();
    let count = segments.len().saturating_sub(1);
    if count > 0 && segments.last().is_some_and(|s| s.trim().is_empty()) {
        count - 1
    } else {
        count
    }
}

// ---------------------------------------------------------------------------
// Byte offset → line number mapping
// ---------------------------------------------------------------------------

fn build_line_offsets(text: &str) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(64);
    offsets.push(0);
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

fn byte_to_line(byte: usize, line_offsets: &[usize]) -> usize {
    match line_offsets.binary_search(&(byte + 1)) {
        Ok(idx) => idx,
        Err(idx) => idx.saturating_sub(1),
    }
}

// ---------------------------------------------------------------------------
// pulldown_cmark::Alignment → j_md::Alignment
// ---------------------------------------------------------------------------

fn convert_alignment(a: pulldown_cmark::Alignment) -> Alignment {
    match a {
        pulldown_cmark::Alignment::None => Alignment::None,
        pulldown_cmark::Alignment::Left => Alignment::Left,
        pulldown_cmark::Alignment::Center => Alignment::Center,
        pulldown_cmark::Alignment::Right => Alignment::Right,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// 将 Markdown 文本解析为平台无关的 IR 文档结构。
pub fn parse_markdown(md: &str) -> ParsedDocument {
    // 预处理：表格分隔行修复（通用 GFM 兼容性）
    let md_owned;
    let md = if needs_table_separator_fix(md) {
        md_owned = normalize_table_separators(md);
        &md_owned as &str
    } else {
        md
    };

    let line_offsets = build_line_offsets(md);

    let options = pulldown_cmark::Options::ENABLE_STRIKETHROUGH
        | pulldown_cmark::Options::ENABLE_TABLES
        | pulldown_cmark::Options::ENABLE_TASKLISTS;
    let parser = pulldown_cmark::Parser::new_ext(md, options);

    let mut ctx = ParseContext::new();

    for (event, range) in parser.into_offset_iter() {
        let source = if !range.is_empty() {
            SourceRange {
                start_line: byte_to_line(range.start, &line_offsets),
                end_line: byte_to_line(range.end.saturating_sub(1), &line_offsets),
            }
        } else {
            SourceRange::default()
        };
        ctx.current_source = source;
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                ctx.flush_paragraph();
                ctx.heading_level = Some(level as u8);
            }
            Event::End(TagEnd::Heading(level)) => {
                let content = std::mem::take(&mut ctx.current_inlines);
                let block = Block {
                    source: ctx.current_source,
                    kind: BlockKind::Heading {
                        level: level as u8,
                        content,
                    },
                };
                ctx.push_block(block);
                ctx.heading_level = None;
            }

            Event::Start(Tag::Strong) => {
                ctx.inline_stack.push(InlineContainer::Strong);
                ctx.inline_children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Strong) => {
                ctx.inline_stack.pop();
                let children = ctx.inline_children_stack.pop().unwrap_or_default();
                ctx.push_inline(Inline::Strong(children));
            }
            Event::Start(Tag::Emphasis) => {
                ctx.inline_stack.push(InlineContainer::Emphasis);
                ctx.inline_children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Emphasis) => {
                ctx.inline_stack.pop();
                let children = ctx.inline_children_stack.pop().unwrap_or_default();
                ctx.push_inline(Inline::Emphasis(children));
            }
            Event::Start(Tag::Strikethrough) => {
                ctx.inline_stack.push(InlineContainer::Strikethrough);
                ctx.inline_children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Strikethrough) => {
                ctx.inline_stack.pop();
                let children = ctx.inline_children_stack.pop().unwrap_or_default();
                ctx.push_inline(Inline::Strikethrough(children));
            }

            Event::Start(Tag::Link { dest_url, .. }) => {
                ctx.inline_stack.push(InlineContainer::Link {
                    url: dest_url.to_string(),
                });
                ctx.inline_children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Link) => {
                let container = ctx.inline_stack.pop();
                let children = ctx.inline_children_stack.pop().unwrap_or_default();
                if let Some(InlineContainer::Link { url }) = container {
                    ctx.push_inline(Inline::Link {
                        text: children,
                        url,
                    });
                }
            }

            Event::Start(Tag::CodeBlock(kind)) => {
                ctx.flush_paragraph();
                ctx.in_code_block = true;
                ctx.code_block_content.clear();
                ctx.code_block_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                let block = Block {
                    source: ctx.current_source,
                    kind: BlockKind::CodeBlock {
                        lang: std::mem::take(&mut ctx.code_block_lang),
                        code: std::mem::take(&mut ctx.code_block_content),
                    },
                };
                ctx.push_block(block);
                ctx.in_code_block = false;
            }

            Event::Code(text) => {
                if ctx.in_table {
                    ctx.table_handle_code(&text);
                } else {
                    ctx.push_inline(Inline::Code(text.to_string()));
                }
            }

            Event::Start(Tag::List(start)) => {
                ctx.flush_paragraph();
                ctx.list_stack.push(ListFrame {
                    ordered: start.is_some(),
                    start_index: start,
                    items: Vec::new(),
                });
            }
            Event::End(TagEnd::List(_)) => {
                ctx.flush_paragraph();
                let Some(frame) = ctx.list_stack.pop() else {
                    continue;
                };
                let block = Block {
                    source: ctx.current_source,
                    kind: BlockKind::List(ListData {
                        ordered: frame.ordered,
                        start_index: frame.start_index,
                        items: frame.items,
                    }),
                };
                ctx.push_block(block);
            }
            Event::Start(Tag::Item) => {
                ctx.item_stack.push(ItemFrame::default());
            }
            Event::End(TagEnd::Item) => {
                let Some(frame) = ctx.item_stack.pop() else {
                    continue;
                };
                let item = ListItem {
                    checked: frame.checked,
                    content: frame.content,
                    children: frame.children,
                };
                if let Some(list) = ctx.list_stack.last_mut() {
                    list.items.push(item);
                }
            }
            Event::TaskListMarker(checked) => {
                if let Some(item) = ctx.item_stack.last_mut() {
                    item.checked = Some(checked);
                }
            }

            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                ctx.flush_paragraph();
            }

            Event::Start(Tag::BlockQuote(_)) => {
                ctx.flush_paragraph();
                ctx.blockquote_stack.push(Vec::new());
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                ctx.flush_paragraph();
                let inner_blocks = ctx.blockquote_stack.pop().unwrap_or_default();
                let block = Block {
                    source: ctx.current_source,
                    kind: BlockKind::BlockQuote(inner_blocks),
                };
                ctx.push_block(block);
            }

            Event::Text(text) => {
                if ctx.image_url.is_some() {
                    ctx.image_alt.push_str(&text);
                } else if ctx.in_code_block {
                    ctx.code_block_content.push_str(&text);
                } else if ctx.in_table {
                    ctx.table_handle_text(&text);
                } else {
                    ctx.push_inline(Inline::Text(text.to_string()));
                }
            }

            Event::SoftBreak => {
                if ctx.in_table {
                    ctx.current_cell_inlines.push(Inline::SoftBreak);
                } else {
                    ctx.push_inline(Inline::SoftBreak);
                }
            }
            Event::HardBreak => {
                if ctx.in_table {
                    ctx.current_cell_inlines.push(Inline::HardBreak);
                } else {
                    ctx.push_inline(Inline::HardBreak);
                }
            }

            Event::Rule => {
                ctx.flush_paragraph();
                ctx.push_block(Block {
                    source: ctx.current_source,
                    kind: BlockKind::Rule,
                });
            }

            Event::Start(Tag::Table(alignments)) => {
                ctx.flush_paragraph();
                ctx.in_table = true;
                ctx.table_rows.clear();
                ctx.table_alignments = alignments.into_iter().map(convert_alignment).collect();
            }
            Event::End(TagEnd::Table) => {
                ctx.flush_paragraph();
                ctx.in_table = false;
                let data = TableData {
                    alignments: std::mem::take(&mut ctx.table_alignments),
                    rows: std::mem::take(&mut ctx.table_rows),
                };
                ctx.push_block(Block {
                    source: ctx.current_source,
                    kind: BlockKind::Table(data),
                });
            }
            Event::Start(Tag::TableHead) => {
                ctx.current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                let row = std::mem::take(&mut ctx.current_row);
                ctx.table_rows.push(row);
            }
            Event::Start(Tag::TableRow) => {
                ctx.current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                let row = std::mem::take(&mut ctx.current_row);
                ctx.table_rows.push(row);
            }
            Event::Start(Tag::TableCell) => {
                ctx.current_cell_inlines.clear();
                ctx.table_inline_stack.clear();
                ctx.table_inline_children_stack.clear();
            }
            Event::End(TagEnd::TableCell) => {
                ctx.table_flush_inline_stack();
                let cell = std::mem::take(&mut ctx.current_cell_inlines);
                ctx.current_row.push(cell);
            }

            Event::Start(Tag::Image { dest_url, .. }) => {
                ctx.flush_paragraph();
                ctx.image_url = Some(dest_url.to_string());
                ctx.image_alt.clear();
            }
            Event::End(TagEnd::Image) => {
                if let Some(_url) = ctx.image_url.take() {
                    // Image handling deferred to future step
                }
                ctx.image_alt.clear();
            }

            _ => {}
        }
    }

    ctx.flush_paragraph();

    ParsedDocument {
        blocks: ctx.blocks,
        line_to_block: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// ParseContext table helpers
// ---------------------------------------------------------------------------

impl ParseContext {
    fn table_handle_text(&mut self, text: &str) {
        self.table_flush_inline_stack();
        self.current_cell_inlines
            .push(Inline::Text(text.to_string()));
    }

    fn table_handle_code(&mut self, text: &str) {
        self.table_flush_inline_stack();
        self.current_cell_inlines
            .push(Inline::Code(text.to_string()));
    }

    fn table_flush_inline_stack(&mut self) {
        while let Some(children) = self.table_inline_children_stack.pop() {
            let container = self.table_inline_stack.pop();
            match container {
                Some(InlineContainer::Strong) => {
                    self.current_cell_inlines.push(Inline::Strong(children));
                }
                Some(InlineContainer::Emphasis) => {
                    self.current_cell_inlines.push(Inline::Emphasis(children));
                }
                Some(InlineContainer::Strikethrough) => {
                    self.current_cell_inlines
                        .push(Inline::Strikethrough(children));
                }
                Some(InlineContainer::Link { url }) => {
                    self.current_cell_inlines.push(Inline::Link {
                        text: children,
                        url,
                    });
                }
                None => {
                    self.current_cell_inlines.extend(children);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::BlockKind;

    #[test]
    fn parse_heading() {
        let doc = parse_markdown("# Hello");
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::Heading { level, content } => {
                assert_eq!(*level, 1);
                assert_eq!(content.len(), 1);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn parse_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let doc = parse_markdown(md);
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::CodeBlock { lang, code } => {
                assert_eq!(lang, "rust");
                assert!(code.contains("fn main()"));
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn parse_inline_formatting() {
        let md = "text **bold** *italic* ~~strike~~ `code`";
        let doc = parse_markdown(md);
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::Paragraph(inlines) => {
                assert!(inlines.len() >= 7); // text + bold + text + italic + text + strike + text + code
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn parse_table_with_alignment() {
        let md = "| A | B |\n| ---: | :--- |\n| 1 | 2 |";
        let doc = parse_markdown(md);
        let table_block = doc
            .blocks
            .iter()
            .find(|b| matches!(&b.kind, BlockKind::Table(_)));
        assert!(table_block.is_some());
        if let Some(Block {
            kind: BlockKind::Table(data),
            ..
        }) = table_block
        {
            assert_eq!(data.alignments.len(), 2);
            assert_eq!(data.alignments[0], Alignment::Right);
            assert_eq!(data.alignments[1], Alignment::Left);
            assert_eq!(data.rows.len(), 2); // header + data row
        }
    }

    #[test]
    fn parse_list_nested() {
        let md = "- item 1\n  - nested\n- item 2";
        let doc = parse_markdown(md);
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::List(data) => {
                assert_eq!(data.items.len(), 2);
                assert_eq!(data.items[0].children.len(), 1);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn parse_blockquote() {
        let md = "> quoted text";
        let doc = parse_markdown(md);
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::BlockQuote(blocks) => {
                assert!(!blocks.is_empty());
            }
            _ => panic!("expected blockquote"),
        }
    }

    #[test]
    fn table_separator_fix() {
        let md = "| A | B | C |\n| --- | --- |\n| 1 | 2 | 3 |";
        let doc = parse_markdown(md);
        let table_block = doc
            .blocks
            .iter()
            .find(|b| matches!(&b.kind, BlockKind::Table(_)));
        assert!(table_block.is_some());
    }
}
