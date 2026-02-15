use gpui::SharedString;

use crate::highlighter::HighlightTheme;

use super::{
    format,
    node::{BlockNode, NodeContext, Span},
};

#[derive(Clone, Debug)]
pub struct MarkdownCodeBlock {
    pub index: usize,
    pub code: SharedString,
    pub language: Option<SharedString>,
    pub span: Option<Span>,
}

pub fn parse_markdown_code_blocks(
    source: &str,
    highlight_theme: &HighlightTheme,
) -> Result<Vec<MarkdownCodeBlock>, SharedString> {
    let mut node_cx = NodeContext::default();
    let document = format::markdown::parse(source, &mut node_cx, highlight_theme)?;

    let blocks = document
        .blocks
        .into_iter()
        .enumerate()
        .filter_map(|(index, block)| match block {
            BlockNode::CodeBlock(code_block) => Some(MarkdownCodeBlock {
                index,
                code: code_block.code(),
                language: code_block.lang(),
                span: code_block.span,
            }),
            _ => None,
        })
        .collect();

    Ok(blocks)
}
