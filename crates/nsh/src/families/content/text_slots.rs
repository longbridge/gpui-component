//! Map parsed Markdown nodes to live NSH slots without serializing subtrees.
use gpui_kit::component::text::{
    InlineElement, InlineRenderContext, MarkdownExtensions, MarkdownNode, MarkdownParseContext,
    MarkdownPlugin, markdown_ast,
};
use gpui_kit::{App, IntoElement, Window, div};
use nsh_core::ui::JsObject;
use nsh_platform_gpui::ComponentContext;
use std::hash::{DefaultHasher, Hash, Hasher};
#[derive(Clone, PartialEq, Eq, Hash, JsObject)]
pub(super) struct TextSlotRule {
    source: String,
    slot: String,
    text: Option<String>,
    offset: Option<usize>,
}
impl TextSlotRule {
    fn matches(&self, node: &markdown_ast::Node, context: &MarkdownParseContext<'_>) -> bool {
        context.node_source(node) == Some(self.source.as_str())
            && self.offset.is_none_or(|offset| {
                node.position()
                    .is_some_and(|position| position.start.offset + context.offset() == offset)
            })
    }
    fn node(&self, name: &str) -> MarkdownNode {
        MarkdownNode::new(name.to_owned(), self.slot.clone())
            .text(self.text.clone().unwrap_or_else(|| self.source.clone()))
            .markdown(self.source.clone())
    }
}
struct InlineSlot {
    name: String,
    rule: TextSlotRule,
    context: ComponentContext,
}
impl MarkdownPlugin for InlineSlot {
    fn name(&self) -> &str {
        &self.name
    }
    fn parse(
        &self,
        node: &markdown_ast::Node,
        context: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        self.rule
            .matches(node, context)
            .then(|| self.rule.node(&self.name))
    }
    fn render_inline(
        &self,
        _: &MarkdownNode,
        _: &InlineRenderContext,
        _: &mut Window,
        _: &mut App,
    ) -> Option<InlineElement> {
        crate::bridge::slot(&self.context, &self.rule.slot).map(InlineElement::new)
    }
}
pub(super) fn extend(
    mut extensions: MarkdownExtensions,
    blocks: &[TextSlotRule],
    inline: &[TextSlotRule],
    frontmatter: bool,
    mdx: bool,
    context: &ComponentContext,
) -> MarkdownExtensions {
    let blocks = blocks.to_vec();
    let inline = inline.to_vec();
    if blocks.is_empty() && inline.is_empty() {
        return extensions;
    }
    let mut revision = DefaultHasher::new();
    blocks.hash(&mut revision);
    inline.hash(&mut revision);
    frontmatter.hash(&mut revision);
    mdx.hash(&mut revision);
    extensions = extensions.parser_revision(revision.finish());
    if !blocks.is_empty() {
        let context = context.clone();
        extensions = extensions
            .block_parser(move |node, parse| {
                blocks
                    .iter()
                    .find(|rule| rule.matches(node, parse))
                    .map(|rule| rule.node("nsh-slot"))
            })
            .block_renderer("nsh-slot", move |node, _, _| {
                node.data::<String>()
                    .and_then(|name| crate::bridge::slot(&context, name))
                    .unwrap_or_else(|| div().into_any_element())
            });
    }
    for (index, rule) in inline.into_iter().enumerate() {
        extensions = extensions.plugin(InlineSlot {
            name: format!("nsh-inline-slot-{index}"),
            rule,
            context: context.clone(),
        });
    }
    extensions
}
