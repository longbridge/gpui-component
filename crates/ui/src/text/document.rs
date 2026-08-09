use gpui::{
    App, InteractiveElement as _, IntoElement, ListState, ParentElement as _, SharedString,
    Styled as _, Window, div,
};

use std::ops::RangeInclusive;

use crate::text::{
    SelectionFormat,
    node::{BlockNode, NodeContext},
};

/// The parsed document AST.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ParsedDocument {
    pub(crate) source: SharedString,
    pub(crate) blocks: Vec<BlockNode>,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct NodeRenderOptions {
    pub(crate) ix: usize,
    pub(crate) in_list: bool,
    pub(crate) todo: bool,
    pub(crate) ordered: bool,
    pub(crate) depth: usize,
    pub(crate) is_last: bool,
}

impl NodeRenderOptions {
    pub(crate) fn is_last(mut self, is_last: bool) -> Self {
        self.is_last = is_last;
        self
    }
}

impl ParsedDocument {
    pub(super) fn text(&self) -> String {
        let mut text = String::new();
        for block in self.blocks.iter() {
            text.push_str(&block.text());
        }
        text
    }

    /// The selected text across all blocks, in `format`.
    ///
    /// In [`SelectionFormat::Source`] each block reconstructs its own Markdown
    /// source (inline markup, and block prefixes for headings and lists), and
    /// top-level blocks are joined with a blank line so the result re-renders
    /// with the same block structure.
    ///
    /// A block only learns its selection when it is painted, so in a scrollable
    /// (virtualized) view every block the user scrolled past reports nothing.
    /// The selection is one continuous range, so blocks it spans that came up
    /// empty are inside it and are emitted whole rather than dropped. `blocks`
    /// bounds that span; it comes from the selection endpoints, which hold on to
    /// their block index even after it scrolls out of view (the painted blocks
    /// alone cannot bound the span, because the press that starts a drag leaves
    /// an empty selection that never reaches paint). Without it, fall back to
    /// the painted blocks. (Source mode is only used by non-scrollable views,
    /// where `blocks` is `None` and every block paints.)
    ///
    /// A standalone image (a paragraph that is only an image) has no selectable
    /// text run, so it never carries a selection of its own. It is therefore
    /// included when it is *enclosed* by the selection — some block before and
    /// some block after it are selected — mirroring how an inline image is
    /// emitted when the selection runs into it. (Select-all returns the source
    /// verbatim, so a leading or trailing image is still copied there.)
    pub(super) fn selected_text(
        &self,
        format: SelectionFormat,
        blocks: Option<RangeInclusive<usize>>,
    ) -> String {
        if format == SelectionFormat::Plain {
            let painted = self
                .blocks
                .iter()
                .map(|block| block.has_selection())
                .collect::<Vec<_>>();
            let (Some(first), Some(last)) = (
                painted.iter().position(|painted| *painted),
                painted.iter().rposition(|painted| *painted),
            ) else {
                return String::new();
            };

            let last_ix = self.blocks.len().saturating_sub(1);
            let (first, last) = match blocks {
                Some(blocks) => (
                    first.min(*blocks.start()),
                    last.max(*blocks.end()).min(last_ix),
                ),
                None => (first, last),
            };

            let mut text = String::new();
            for (ix, block) in self.blocks.iter().enumerate().take(last + 1).skip(first) {
                let selected = block.selected_text(format);
                if !selected.is_empty() {
                    text.push_str(&selected);
                } else if !painted[ix] {
                    // Never painted, so it cannot report a selection of its own
                    // even though the span covers it. A painted block that came
                    // up empty really has nothing selected, and stays empty.
                    text.push_str(&block.text());
                }
            }
            return text;
        }

        let mut blocks: Vec<String> = Vec::new();
        // Standalone-image blocks seen since the last selected block, held back
        // until a following block turns out to be selected (enclosure).
        let mut pending_images: Vec<String> = Vec::new();
        for block in self.blocks.iter() {
            if let BlockNode::Paragraph(paragraph) = block
                && paragraph.is_only_images()
                && block.selected_text(format).trim().is_empty()
            {
                // Only a candidate if something before it is already selected;
                // otherwise a leading unselected image would leak in.
                if !blocks.is_empty() {
                    pending_images.push(paragraph.images_markdown());
                }
                continue;
            }

            let source = block.selected_text(format);
            let trimmed = source.trim_end_matches('\n');
            if !trimmed.is_empty() {
                // A selected block follows the pending images, so they were
                // enclosed by the selection: flush them before this block.
                blocks.append(&mut pending_images);
                blocks.push(trimmed.to_string());
            }
        }
        // Any images still pending had no selected block after them, so they
        // were not enclosed; drop them.
        blocks.join("\n\n")
    }

    /// Synchronously clear the selection stored in every inline state.
    ///
    /// This mirrors the [`selected_text`](Self::selected_text) traversal so the
    /// stored selection can be cleared without relying on a repaint. Offscreen
    /// (virtualized) views do not repaint, so their `InlineState.selection`
    /// would otherwise retain stale values from the last painted frame.
    pub(super) fn clear_selection(&self) {
        for block in self.blocks.iter() {
            block.clear_selection();
        }
    }

    /// Converts the node to markdown format.
    ///
    /// This is used to generate markdown for test.
    #[allow(dead_code)]
    pub(crate) fn to_markdown(&self) -> String {
        self.blocks
            .iter()
            .map(|child| child.to_markdown())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub(super) fn render_root(
        &self,
        list_state: Option<ListState>,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let Some(list_state) = list_state else {
            let blocks_len = self.blocks.len();
            return div()
                .id("document")
                .children(self.blocks.iter().enumerate().map(move |(ix, node)| {
                    let is_last = ix + 1 == blocks_len;
                    node.render_block(
                        NodeRenderOptions {
                            ix,
                            is_last,
                            ..Default::default()
                        },
                        node_cx,
                        window,
                        cx,
                    )
                }));
        };

        let options = NodeRenderOptions {
            is_last: true,
            ..Default::default()
        };

        let blocks = &self.blocks;

        if list_state.item_count() != blocks.len() {
            list_state.reset(blocks.len());
        }

        div().id("document").size_full().child(
            gpui::list(list_state, {
                let node_cx = node_cx.clone();
                let blocks = blocks.clone();
                move |ix, window, cx| {
                    let is_last = ix + 1 == blocks.len();
                    blocks[ix]
                        .render_block(
                            NodeRenderOptions {
                                ix,
                                is_last,
                                ..options
                            },
                            &node_cx,
                            window,
                            cx,
                        )
                        .into_any_element()
                }
            })
            .size_full(),
        )
    }
}
