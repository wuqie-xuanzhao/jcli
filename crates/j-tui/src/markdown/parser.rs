#[cfg(test)]
mod tests;

use crate::markdown::ir::ParsedDocument;
use crate::util::text::{needs_terminal_sanitization, sanitize_terminal_text};

// ---------------------------------------------------------------------------
// Public API: parse_markdown (TUI facade — adds terminal sanitization)
// ---------------------------------------------------------------------------

/// 将 Markdown 文本解析为 IR 文档结构（TUI 版本，含终端字符清洗预处理）。
/// 预处理后委托 j-md 的纯解析器。
pub fn parse_markdown(md: &str, max_width: usize) -> ParsedDocument {
    // 预处理：ANSI/OSC/tab/carriage return/控制字符清洗
    let normalized_md;
    let md = if needs_terminal_sanitization(md) {
        normalized_md = sanitize_terminal_text(md);
        normalized_md.as_str()
    } else {
        md
    };

    // 预处理：中文引号与加粗标记的零宽空格（TUI 渲染 workaround）
    let md_owned;
    let md = if md.contains("**\u{201C}")
        || md.contains("**\u{2018}")
        || md.contains("\u{201D}**")
        || md.contains("\u{2019}**")
    {
        md_owned = md
            .replace("**\u{201C}", "**\u{200B}\u{201C}")
            .replace("**\u{2018}", "**\u{200B}\u{2018}")
            .replace("\u{201D}**", "\u{201D}\u{200B}**")
            .replace("\u{2019}**", "\u{2019}\u{200B}**");
        &md_owned as &str
    } else {
        md
    };

    // max_width 参数当前未用于解析（仅用于渲染时的宽度计算）
    let _ = max_width;

    // 委托 j-md 纯解析器（内部已处理表格分隔行修复等通用预处理）
    j_md::parse_markdown(md)
}

// ---------------------------------------------------------------------------
// Public API: markdown_to_lines (Facade — unchanged signature)
// ---------------------------------------------------------------------------

use crate::markdown::ir::TableData;
use crate::markdown::render::render_document_wrapped;
use crate::markdown::theme::MdStyle;
use ratatui::text::Line;

/// 将 Markdown 文本渲染为 TUI 可显示的 `Line` 列表，应用主题着色和自动换行。
pub fn markdown_to_lines(md: &str, max_width: usize, theme: &dyn MdStyle) -> Vec<Line<'static>> {
    let content_width = max_width.saturating_sub(2);
    let doc = parse_markdown(md, content_width);
    render_document_wrapped(&doc, theme, content_width)
}

/// 从源码行切片中解析表格为 `TableData`。
pub fn parse_table_from_source(table_lines: &[&str]) -> Option<TableData> {
    use crate::markdown::ir::BlockKind;

    let md: String = table_lines.to_vec().join("\n");
    let doc = parse_markdown(&md, usize::MAX);

    for block in &doc.blocks {
        if let BlockKind::Table(data) = &block.kind {
            return Some(data.clone());
        }
    }

    None
}

#[cfg(test)]
mod bench_table {
    use super::*;

    #[test]
    fn bench_parse_table_from_source() {
        let table_lines: &[&str] = &[
            "| 功能 | 状态 | 备注 |",
            "| --- | --- | --- |",
            "| **内联语法** | ✓ | `code` 支持 |",
            "| 表格渲染 | ✓ | 共享层 |",
            "| 性能优化 | - | 待评估 |",
        ];

        let _ = parse_table_from_source(table_lines);

        let iterations = 1000;
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            let _ = parse_table_from_source(table_lines);
        }
        let elapsed = start.elapsed();
        let per_call_us = elapsed.as_micros() as f64 / iterations as f64;

        eprintln!(
            "\n=== parse_table_from_source 性能 ===\n\
             总调用: {} 次\n\
             总耗时: {:.2} ms\n\
             单次耗时: {:.1} μs",
            iterations,
            elapsed.as_millis(),
            per_call_us,
        );

        assert!(
            per_call_us < 200.0,
            "parse_table_from_source 单次耗时 {:.1}μs 超过 200μs 阈值",
            per_call_us
        );
    }
}
