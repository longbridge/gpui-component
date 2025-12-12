use gpui::{
    App, InteractiveElement as _, IntoElement, ListState, ParentElement as _, SharedString,
    Styled as _, Window, div,
};

use crate::text::node::{BlockNode, NodeContext};

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
    pub(super) fn selected_text(&self) -> String {
        let mut text = String::new();
        for block in self.blocks.iter() {
            text.push_str(&block.selected_text());
        }
        text
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
        let options = NodeRenderOptions {
            is_last: true,
            ..Default::default()
        };

        let Some(list_state) = list_state else {
            return div()
                .id("document")
                .children(self.blocks.iter().enumerate().map(move |(ix, node)| {
                    node.render_block(NodeRenderOptions { ix, ..options }, node_cx, window, cx)
                }));
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
