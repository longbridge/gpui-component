//! Host API for external editors.
//!
//! An editor that lives outside gpui-kit keeps its own document model and
//! hands gpui-kit a [`BlockNode`] tree, the same tree [`TextView`] renders.
//! gpui-kit renders it and gives back, per text leaf, the geometry the editor
//! needs for hit-testing, caret placement and platform input (IME) bounds.
//!
//! The boundary is deliberately narrow:
//!
//! - **In:** a [`BlockNode`] tree whose text leaves carry a host-chosen `u64`
//!   key (see [`Paragraph::with_leaf_key`] and [`CodeBlock::with_leaf_key`]),
//!   a [`RenderContext`], and per-key decoration ranges.
//! - **Out:** one opaque [`LeafGeometry`] per key from
//!   [`collect_leaf_geometry`].
//!
//! Nothing lower is exposed: no `TextLayout`, no layout internals, no sink
//! trait, no retained-layout or projection caches.
//!
//! This is a host API for external editors. It is pre-1.0 and may change
//! between gpui-kit releases.
//!
//! [`TextView`]: super::TextView

use std::{cell::RefCell, collections::HashMap, ops::Range, rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, Bounds, Hsla, Pixels, Point, SharedString, TextLayout, Window, point, px,
};

use super::{
    TextViewDefaults, TextViewStyle,
    document::{NodeRenderOptions, PrevBlock, flow_position},
    editor_seam::{self, EditorSink},
    format::markdown,
    markdown_ext::MarkdownExtensions,
    node::NodeContext,
};

pub use super::node::{
    BlockNode, BlockState, CodeBlock, ColumnumnAlign as ColumnAlign, ImageNode, InlineNode,
    LinkMark, Paragraph, Span, Table, TextMark,
};

/// Parses Markdown with the parser [`TextView`](super::TextView) uses,
/// including the given extensions, into the block tree.
pub fn parse_markdown(
    source: &str,
    extensions: Arc<MarkdownExtensions>,
) -> Result<Vec<BlockNode>, SharedString> {
    let mut cx = NodeContext {
        markdown_extensions: extensions,
        ..Default::default()
    };
    let parsed = markdown::parse(source, &mut cx)?;
    Ok(Arc::unwrap_or_clone(parsed.blocks))
}

/// What [`BlockNode::render`] needs besides the node: the text style, code
/// highlighter and Markdown extensions.
#[derive(Clone)]
pub struct RenderContext {
    node: NodeContext,
}

impl RenderContext {
    /// The context TextView would start from: the global [`TextViewDefaults`]
    /// style and code-block highlighter when set, else the theme's style.
    pub fn from_app(cx: &App) -> Self {
        let theme = crate::Theme::global(cx);
        let defaults = cx.try_global::<TextViewDefaults>();
        let style = defaults
            .and_then(|defaults| defaults.style().cloned())
            .unwrap_or_else(|| TextViewStyle::from_theme(&theme));
        Self {
            node: NodeContext {
                style: Arc::new(style),
                code_block_highlighter: defaults.and_then(|d| d.code_block_highlighter()),
                ..Default::default()
            },
        }
    }

    /// Replaces the text style.
    pub fn with_style(mut self, style: TextViewStyle) -> Self {
        self.node.style = Arc::new(style);
        self
    }

    /// Sets the Markdown extensions that render custom nodes.
    pub fn with_markdown_extensions(mut self, extensions: Arc<MarkdownExtensions>) -> Self {
        self.node.markdown_extensions = extensions;
        self
    }

    /// Sets the font size body text inherits, which the block gaps and
    /// heading sizes are relative to.
    pub fn with_body_font_size(mut self, size: Pixels) -> Self {
        self.node.body_font_size = size;
        self
    }

    /// The text style blocks render with.
    pub fn style(&self) -> &TextViewStyle {
        &self.node.style
    }
}

/// Where a block stands in the flow it is rendered in, which decides the gaps
/// around it: the first block takes no gap above, the last none below, and a
/// heading or rule counts the block before it.
#[derive(Clone, Copy, Debug)]
pub struct FlowPosition {
    is_first: bool,
    is_last: bool,
    prev: PrevBlock,
}

impl FlowPosition {
    /// The position of `blocks[ix]` among its siblings. Blocks that render
    /// nothing (definitions, breaks) are skipped when looking for the block
    /// before it.
    pub fn at(blocks: &[&BlockNode], ix: usize, is_last: bool) -> Self {
        let (is_first, prev) = flow_position(blocks, ix);
        Self {
            is_first,
            is_last,
            prev,
        }
    }

    /// Overrides whether the block opens its flow, e.g. for a chunk of a
    /// long list rendered as a list of its own.
    pub fn first(mut self, is_first: bool) -> Self {
        self.is_first = is_first;
        self
    }

    /// Overrides whether the block closes its flow.
    pub fn last(mut self, is_last: bool) -> Self {
        self.is_last = is_last;
        self
    }
}

impl BlockNode {
    /// Renders the block as TextView does. `ix` identifies the block among
    /// its siblings (element ids derive from it).
    pub fn render(
        &self,
        ix: usize,
        flow: FlowPosition,
        cx: &RenderContext,
        window: &mut Window,
        app: &mut App,
    ) -> AnyElement {
        self.render_block(
            NodeRenderOptions {
                ix,
                is_first: flow.is_first,
                is_last: flow.is_last,
                prev: flow.prev,
                ..Default::default()
            },
            &cx.node,
            window,
            app,
        )
    }
}

/// Runs `f`, which should prepaint rendered blocks, and returns its result
/// with the geometry every keyed text leaf reported meanwhile.
///
/// `decorations` maps a leaf key to byte ranges of that leaf's text (atoms
/// count as one `U+FFFC` each) to paint behind the text: the host's selection,
/// find matches, anything. Leaves without a key, or without an entry, paint as
/// usual. Scopes nest, and the previous one is restored even if `f` panics.
pub fn collect_leaf_geometry<R>(
    decorations: HashMap<u64, Vec<(Range<usize>, Hsla)>>,
    f: impl FnOnce() -> R,
) -> (R, HashMap<u64, LeafGeometry>) {
    let collector = Rc::new(Collector {
        pieces: RefCell::default(),
        decorations,
    });
    let result = editor_seam::with_sink(collector.clone(), f);
    let mut pieces = collector.pieces.take();
    pieces.retain(|_, pieces| !pieces.is_empty());
    let leaves = pieces
        .into_iter()
        .map(|(key, pieces)| (key, LeafGeometry { pieces }))
        .collect();
    (result, leaves)
}

struct Collector {
    pieces: RefCell<HashMap<u64, Vec<Piece>>>,
    decorations: HashMap<u64, Vec<(Range<usize>, Hsla)>>,
}

impl EditorSink for Collector {
    fn report(&self, key: u64, layout: &TextLayout) {
        self.pieces
            .borrow_mut()
            .insert(key, vec![Piece::Whole(layout.clone())]);
    }

    fn begin_pieces(&self, key: u64) {
        self.pieces.borrow_mut().insert(key, Vec::new());
    }

    fn report_text_piece(&self, key: u64, start: usize, layout: &TextLayout) {
        self.pieces
            .borrow_mut()
            .entry(key)
            .or_default()
            .push(Piece::Text {
                start,
                layout: layout.clone(),
            });
    }

    fn report_box_piece(&self, key: u64, range: Range<usize>, bounds: Bounds<Pixels>) {
        self.pieces
            .borrow_mut()
            .entry(key)
            .or_default()
            .push(Piece::Atom { range, bounds });
    }

    fn backgrounds(&self, key: u64) -> Vec<(Range<usize>, Hsla)> {
        self.decorations.get(&key).cloned().unwrap_or_default()
    }
}

/// How a leaf was laid out: one layout for the whole leaf, or (for a mixed
/// paragraph rendered as a flow) single-line text fragments and atom boxes in
/// leaf byte order.
#[derive(Clone)]
enum Piece {
    Whole(TextLayout),
    Text {
        start: usize,
        layout: TextLayout,
    },
    Atom {
        range: Range<usize>,
        bounds: Bounds<Pixels>,
    },
}

impl Piece {
    fn bounds(&self) -> Bounds<Pixels> {
        match self {
            Piece::Whole(layout) | Piece::Text { layout, .. } => layout.bounds(),
            Piece::Atom { bounds, .. } => *bounds,
        }
    }

    fn range(&self) -> Range<usize> {
        match self {
            Piece::Whole(layout) => 0..layout.len(),
            Piece::Text { start, layout } => *start..start + layout.len(),
            Piece::Atom { range, .. } => range.clone(),
        }
    }
}

/// Where one keyed text leaf was laid out in the last frame.
///
/// Indices are byte offsets into the leaf's text, where each inline atom
/// (image, custom node) is one `U+FFFC` (3 bytes). Coordinates are window
/// pixels and are valid for the frame that produced them. It is a value, not a
/// handle: it holds no entity or layout and can be kept until the next frame.
#[derive(Clone)]
pub struct LeafGeometry {
    pieces: Vec<Piece>,
}

impl LeafGeometry {
    fn whole(&self) -> Option<&TextLayout> {
        match self.pieces.as_slice() {
            [Piece::Whole(layout)] => Some(layout),
            _ => None,
        }
    }

    /// The bounds of the leaf: the union of its pieces.
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.pieces
            .iter()
            .map(Piece::bounds)
            .reduce(|a, b| a.union(&b))
            .unwrap_or_default()
    }

    /// The length of the leaf's text in bytes, atoms included.
    pub fn len(&self) -> usize {
        self.pieces
            .iter()
            .map(|piece| piece.range().end)
            .max()
            .unwrap_or(0)
    }

    /// Whether the leaf's text is empty (it still has a line box).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The height of a line.
    pub fn line_height(&self) -> Pixels {
        self.pieces
            .iter()
            .find_map(|piece| match piece {
                Piece::Whole(layout) | Piece::Text { layout, .. } => Some(layout.line_height()),
                Piece::Atom { .. } => None,
            })
            .unwrap_or_else(|| self.bounds().size.height)
    }

    /// The height of a caret: the font's ascent plus descent, not the whole
    /// line box, which includes leading.
    pub fn caret_height(&self) -> Pixels {
        let line_height = self.line_height();
        self.pieces
            .iter()
            .find_map(|piece| match piece {
                Piece::Whole(layout) | Piece::Text { layout, .. } => {
                    let line = layout.line_layout_for_index(0)?;
                    let h = line.unwrapped_layout.ascent + line.unwrapped_layout.descent;
                    (h > Pixels::ZERO).then_some(h)
                }
                Piece::Atom { .. } => None,
            })
            .unwrap_or(line_height * 0.85)
            .min(line_height)
    }

    /// The top of a caret placed at byte `index`, in window coordinates.
    pub fn position_for_index(&self, index: usize) -> Point<Pixels> {
        if let Some(layout) = self.whole() {
            return layout
                .position_for_index(index.min(layout.len()))
                .unwrap_or_else(|| self.bounds().origin);
        }
        // The piece the index starts, else the one it ends (a caret at a
        // fragment boundary sits at the start of the next fragment).
        let piece = self
            .pieces
            .iter()
            .find(|p| p.range().contains(&index))
            .or_else(|| self.pieces.iter().rev().find(|p| p.range().end == index))
            .or(self.pieces.last());
        let Some(piece) = piece else {
            return self.bounds().origin;
        };
        match piece {
            Piece::Whole(layout) | Piece::Text { layout, .. } => {
                let local = index.saturating_sub(piece.range().start).min(layout.len());
                layout
                    .position_for_index(local)
                    .unwrap_or_else(|| layout.bounds().origin)
            }
            Piece::Atom { range, bounds } => {
                let x = if index >= range.end {
                    bounds.right()
                } else {
                    bounds.left()
                };
                // Align with the text line: an atom taller than the line
                // keeps the caret at its bottom line.
                let y = (bounds.bottom() - self.line_height()).max(bounds.top());
                point(x, y)
            }
        }
    }

    /// The byte index of the caret position closest to `at`: glyph halves
    /// round to the nearer side, unlike `TextLayout::index_for_position`.
    pub fn index_for_position(&self, at: Point<Pixels>) -> usize {
        if let Some(layout) = self.whole() {
            return whole_index_at(layout, at);
        }
        // The visual line under the point (pieces whose vertical extent
        // holds it, else the nearest), then the horizontally nearest piece.
        let dy = |p: &Piece| {
            let b = p.bounds();
            if at.y < b.top() {
                b.top() - at.y
            } else if at.y > b.bottom() {
                at.y - b.bottom()
            } else {
                px(0.)
            }
        };
        let dx = |p: &Piece| {
            let b = p.bounds();
            if at.x < b.left() {
                b.left() - at.x
            } else if at.x > b.right() {
                at.x - b.right()
            } else {
                px(0.)
            }
        };
        let Some(piece) = self.pieces.iter().min_by(|a, b| {
            (dy(a), dx(a))
                .partial_cmp(&(dy(b), dx(b)))
                .unwrap_or(std::cmp::Ordering::Equal)
        }) else {
            return 0;
        };
        let index = match piece {
            Piece::Whole(layout) => whole_index_at(layout, at),
            // Fragments are one line: take the caret position nearest to the
            // point (`closest_index_for_position` rounds up at an exact
            // boundary inside scaled code text).
            Piece::Text { start, layout } => {
                let nearest = (0..=layout.len())
                    .filter_map(|i| {
                        layout
                            .position_for_index(i)
                            .map(|p| (i, (p.x - at.x).abs()))
                    })
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map_or(0, |(i, _)| i);
                start + nearest
            }
            Piece::Atom { range, bounds } => {
                if at.x < bounds.center().x {
                    range.start
                } else {
                    range.end
                }
            }
        };
        index.min(self.len())
    }

    /// The byte range of the inline atom drawn under `at`, if any.
    pub fn atom_at(&self, at: Point<Pixels>) -> Option<Range<usize>> {
        self.pieces.iter().find_map(|piece| match piece {
            Piece::Atom { range, bounds } if bounds.contains(&at) => Some(range.clone()),
            _ => None,
        })
    }
}

/// The closest index in a whole-leaf layout for a window point.
fn whole_index_at(layout: &TextLayout, at: Point<Pixels>) -> usize {
    let bounds = layout.bounds();
    let line_height = layout.line_height();
    let mut top = bounds.top();
    let mut start = 0;
    let lines = layout.line_layouts();
    for (ix, line) in lines.iter().enumerate() {
        let height = line.size(line_height).height.max(line_height);
        let is_last = ix + 1 == lines.len();
        if at.y < top + height || is_last {
            let local = point(
                (at.x - bounds.left()).max(px(0.)),
                (at.y - top).clamp(px(0.), (height - px(1.)).max(px(0.))),
            );
            let index = match line.closest_index_for_position(local, line_height) {
                Ok(ix) | Err(ix) => ix,
            };
            return start + index;
        }
        top += height;
        start += line.len() + 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use gpui::{
        Context, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
        ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, div, rgb, rgba,
    };

    use super::*;
    use crate::text::{InlineElement, MarkdownNode, markdown_ext::TestInlinePlugin};

    type Geometries = Rc<RefCell<HashMap<u64, LeafGeometry>>>;

    /// Stands in for an editor's root element: prepaints its child inside a
    /// `collect_leaf_geometry` scope and keeps what the leaves reported.
    struct Probe {
        child: Option<AnyElement>,
        decorations: HashMap<u64, Vec<(Range<usize>, Hsla)>>,
        out: Geometries,
        /// Replaces `collect_leaf_geometry` with a custom sink.
        sink: Option<Rc<dyn EditorSink>>,
    }

    impl IntoElement for Probe {
        type Element = Self;
        fn into_element(self) -> Self::Element {
            self
        }
    }

    impl Element for Probe {
        type RequestLayoutState = AnyElement;
        type PrepaintState = ();

        fn id(&self) -> Option<ElementId> {
            None
        }

        fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
            None
        }

        fn request_layout(
            &mut self,
            _: Option<&GlobalElementId>,
            _: Option<&InspectorElementId>,
            window: &mut Window,
            cx: &mut App,
        ) -> (LayoutId, AnyElement) {
            let mut child = self.child.take().unwrap();
            (child.request_layout(window, cx), child)
        }

        fn prepaint(
            &mut self,
            _: Option<&GlobalElementId>,
            _: Option<&InspectorElementId>,
            _: Bounds<Pixels>,
            child: &mut AnyElement,
            window: &mut Window,
            cx: &mut App,
        ) {
            if let Some(sink) = self.sink.take() {
                editor_seam::with_sink(sink, || child.prepaint(window, cx));
                return;
            }
            let decorations = std::mem::take(&mut self.decorations);
            let (_, geometries) = collect_leaf_geometry(decorations, || child.prepaint(window, cx));
            *self.out.borrow_mut() = geometries;
        }

        fn paint(
            &mut self,
            _: Option<&GlobalElementId>,
            _: Option<&InspectorElementId>,
            _: Bounds<Pixels>,
            child: &mut AnyElement,
            _: &mut (),
            window: &mut Window,
            cx: &mut App,
        ) {
            child.paint(window, cx);
        }
    }

    struct Host {
        blocks: Vec<BlockNode>,
        extensions: Arc<MarkdownExtensions>,
        decorations: HashMap<u64, Vec<(Range<usize>, Hsla)>>,
        out: Geometries,
        width: Pixels,
        sink: Option<Rc<dyn EditorSink>>,
    }

    impl Render for Host {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let rcx = RenderContext::from_app(cx)
                .with_markdown_extensions(self.extensions.clone())
                .with_body_font_size(px(16.));
            let refs: Vec<&BlockNode> = self.blocks.iter().collect();
            let children: Vec<AnyElement> = refs
                .iter()
                .enumerate()
                .map(|(ix, block)| {
                    let flow = FlowPosition::at(&refs, ix, ix + 1 == refs.len());
                    block.render(ix, flow, &rcx, window, cx)
                })
                .collect();
            Probe {
                child: Some(div().w(self.width).children(children).into_any_element()),
                decorations: self.decorations.clone(),
                out: self.out.clone(),
                sink: self.sink.clone(),
            }
        }
    }

    fn draw(
        cx: &mut TestAppContext,
        blocks: Vec<BlockNode>,
        decorations: HashMap<u64, Vec<(Range<usize>, Hsla)>>,
        extensions: Arc<MarkdownExtensions>,
        width: Pixels,
    ) -> HashMap<u64, LeafGeometry> {
        cx.update(crate::init);
        let out = Geometries::default();
        let (_, vcx) = cx.add_window_view({
            let out = out.clone();
            move |_, _| Host {
                blocks,
                extensions,
                decorations,
                out,
                width,
                sink: None,
            }
        });
        settle(vcx);
        out.take()
    }

    fn settle(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
    }

    fn plain(text: &str) -> Paragraph {
        let mut paragraph = Paragraph::default();
        paragraph.push(InlineNode::new(text.to_string()));
        paragraph
    }

    fn atom_plugin() -> Arc<MarkdownExtensions> {
        Arc::new(
            MarkdownExtensions::default().plugin(TestInlinePlugin::new("atom").render_with(
                |_, _, _, _| {
                    Some(InlineElement::new(div().w(px(24.)).h(px(16.))).with_baseline(px(12.)))
                },
            )),
        )
    }

    fn atom() -> InlineNode {
        InlineNode::custom(MarkdownNode::new("atom", ()).text("\u{FFFC}"))
    }

    #[gpui::test]
    fn keyed_leaves_report_geometry_and_unkeyed_leaves_do_not(cx: &mut TestAppContext) {
        let blocks = vec![
            BlockNode::Heading {
                level: 1,
                children: plain("Title").with_leaf_key(1),
                span: None,
            },
            BlockNode::Paragraph(plain("first").with_leaf_key(2)),
            BlockNode::Paragraph(plain("not an editor leaf")),
            BlockNode::CodeBlock(
                CodeBlock::new("let x = 1;".into(), Some("rust".into()), None::<Span>)
                    .with_leaf_key(3),
            ),
            BlockNode::Table(Table::new(
                vec![
                    vec![plain("a").with_leaf_key(4), plain("b").with_leaf_key(5)],
                    vec![plain("c").with_leaf_key(6), plain("d").with_leaf_key(7)],
                ],
                vec![ColumnAlign::None, ColumnAlign::Right],
            )),
            BlockNode::List {
                children: vec![BlockNode::ListItem {
                    children: vec![BlockNode::Paragraph(plain("item").with_leaf_key(8))],
                    spread: false,
                    checked: None,
                    span: None,
                }],
                ordered: false,
                start: None,
                span: None,
            },
        ];
        let geometries = draw(cx, blocks, HashMap::new(), Default::default(), px(300.));
        let mut keys: Vec<u64> = geometries.keys().copied().collect();
        keys.sort_unstable();
        assert_eq!(keys, (1..=8).collect::<Vec<_>>());
        for key in [1, 2, 3, 8] {
            assert!(geometries[&key].bounds().size.height > px(0.), "leaf {key}");
        }
        assert_eq!(geometries[&1].len(), "Title".len());
        assert_eq!(geometries[&3].len(), "let x = 1;".len());
        // Document order is top to bottom.
        let top = |key: u64| geometries[&key].bounds().top();
        assert!(top(1) < top(2) && top(2) < top(3) && top(3) < top(4) && top(7) < top(8));
        // Cells of one row share a line; the second column sits to the right.
        assert_eq!(top(4), top(5));
        assert!(geometries[&5].bounds().left() > geometries[&4].bounds().right() - px(1.));
    }

    #[gpui::test]
    fn wrapped_leaf_maps_indices_and_positions(cx: &mut TestAppContext) {
        let text = "the quick brown fox jumps over the lazy dog and keeps running far away";
        let blocks = vec![BlockNode::Paragraph(plain(text).with_leaf_key(1))];
        let geometries = draw(cx, blocks, HashMap::new(), Default::default(), px(120.));
        let leaf = &geometries[&1];
        assert_eq!(leaf.len(), text.len());
        assert!(!leaf.is_empty());
        let line = leaf.line_height();
        assert!(line > px(0.) && leaf.caret_height() <= line);
        let mut last_y = px(-1.);
        let mut lines = std::collections::BTreeSet::new();
        for index in 0..=text.len() {
            let caret = leaf.position_for_index(index);
            assert!(caret.y >= last_y, "carets never move up");
            last_y = caret.y;
            lines.insert((caret.y.as_f32() * 100.) as i64);
            // Hit-testing at a caret lands on that caret (at a soft wrap the
            // end of a line and the start of the next share a caret index).
            let hit = leaf.index_for_position(point(caret.x, caret.y + line / 2.));
            assert_eq!(
                leaf.position_for_index(hit),
                caret,
                "index {index} hit {hit}"
            );
        }
        assert!(lines.len() >= 3, "the text wraps onto several lines");
        // Far outside the leaf clamps to its ends.
        assert_eq!(leaf.index_for_position(point(px(-500.), px(-500.))), 0);
        assert_eq!(
            leaf.index_for_position(point(px(5000.), px(5000.))),
            text.len()
        );
    }

    #[gpui::test]
    fn mixed_text_and_atoms_report_boxes(cx: &mut TestAppContext) {
        let mut paragraph = Paragraph::default();
        paragraph.push(InlineNode::new("ab"));
        paragraph.push(atom());
        paragraph.push(InlineNode::new("cd"));
        let atom_len = '\u{FFFC}'.len_utf8();
        let blocks = vec![BlockNode::Paragraph(paragraph.with_leaf_key(1))];
        let geometries = draw(cx, blocks, HashMap::new(), atom_plugin(), px(300.));
        let leaf = &geometries[&1];
        assert_eq!(leaf.len(), 2 + atom_len + 2);
        // The box is hit-testable, and only the box.
        let range = 2..2 + atom_len;
        let before = leaf.position_for_index(2);
        let after = leaf.position_for_index(2 + atom_len);
        assert!(after.x > before.x, "the caret moves across the atom");
        let inside = point(
            (before.x + after.x) / 2.,
            before.y + leaf.line_height() / 2.,
        );
        assert_eq!(leaf.atom_at(inside), Some(range.clone()));
        assert_eq!(leaf.atom_at(point(before.x - px(8.), inside.y)), None);
        // Clicking either half of the atom puts the caret on its near edge.
        let quarter = |t: f32| point(before.x + (after.x - before.x) * t, inside.y);
        assert_eq!(leaf.index_for_position(quarter(0.25)), range.start);
        assert_eq!(leaf.index_for_position(quarter(0.75)), range.end);
        // Text on both sides keeps its own indices.
        let end = leaf.position_for_index(leaf.len());
        assert_eq!(
            leaf.index_for_position(point(end.x + px(40.), inside.y)),
            leaf.len()
        );
        assert_eq!(leaf.index_for_position(point(px(-40.), inside.y)), 0);
    }

    #[gpui::test]
    fn empty_keyed_leaf_has_a_line_box_and_empty_unkeyed_leaf_stays_empty(cx: &mut TestAppContext) {
        let blocks = vec![
            BlockNode::Paragraph(Paragraph::default()),
            BlockNode::Paragraph(Paragraph::default().with_leaf_key(9)),
        ];
        let geometries = draw(cx, blocks, HashMap::new(), Default::default(), px(300.));
        assert_eq!(geometries.len(), 1);
        let leaf = &geometries[&9];
        assert!(leaf.is_empty());
        assert!(leaf.bounds().size.height > px(0.));
        assert_eq!(leaf.position_for_index(0).x, leaf.bounds().left());
        assert_eq!(
            leaf.index_for_position(point(px(50.), leaf.bounds().top())),
            0
        );
    }

    #[gpui::test]
    fn image_only_leaf_reports_its_box(cx: &mut TestAppContext) {
        let image = ImageNode {
            url: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='20' height='20'/>".into(),
            width: Some(px(20.).into()),
            height: Some(px(20.).into()),
            ..Default::default()
        };
        let mut paragraph = Paragraph::default();
        paragraph.push(InlineNode::image(image));
        let blocks = vec![BlockNode::Paragraph(paragraph.with_leaf_key(1))];
        let geometries = draw(cx, blocks, HashMap::new(), Default::default(), px(300.));
        let leaf = &geometries[&1];
        let atom_len = '\u{FFFC}'.len_utf8();
        assert_eq!(leaf.len(), atom_len);
        let inside = leaf.bounds().center();
        assert_eq!(leaf.atom_at(inside), Some(0..atom_len));
    }

    /// Records which leaves ask for backgrounds, to prove decorations are
    /// requested for keyed leaves only.
    #[derive(Default)]
    struct Recorder {
        asked: RefCell<Vec<u64>>,
    }

    impl EditorSink for Recorder {
        fn report(&self, _: u64, _: &TextLayout) {}
        fn begin_pieces(&self, _: u64) {}
        fn report_text_piece(&self, _: u64, _: usize, _: &TextLayout) {}
        fn report_box_piece(&self, _: u64, _: Range<usize>, _: Bounds<Pixels>) {}
        fn backgrounds(&self, key: u64) -> Vec<(Range<usize>, Hsla)> {
            self.asked.borrow_mut().push(key);
            vec![(0..1, rgb(0xff0000).into())]
        }
    }

    #[gpui::test]
    fn decorations_are_requested_per_keyed_leaf(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let recorder = Rc::new(Recorder::default());
        let mut mixed = Paragraph::default();
        mixed.push(InlineNode::new("ab"));
        mixed.push(atom());
        let blocks = vec![
            BlockNode::Paragraph(plain("one").with_leaf_key(1)),
            BlockNode::Paragraph(plain("unkeyed")),
            BlockNode::Paragraph(mixed.with_leaf_key(2)),
        ];
        let (_, vcx) = cx.add_window_view({
            let sink: Rc<dyn EditorSink> = recorder.clone();
            move |_, _| Host {
                blocks,
                extensions: atom_plugin(),
                decorations: HashMap::new(),
                out: Geometries::default(),
                width: px(300.),
                sink: Some(sink),
            }
        });
        settle(vcx);
        let asked = recorder.asked.borrow();
        assert!(asked.contains(&1) && asked.contains(&2), "{asked:?}");
        assert!(asked.iter().all(|key| *key == 1 || *key == 2), "{asked:?}");
    }

    #[gpui::test]
    fn decorated_leaves_still_report_geometry(cx: &mut TestAppContext) {
        let blocks = vec![BlockNode::Paragraph(plain("hello world").with_leaf_key(1))];
        let decorations = HashMap::from([(
            1,
            vec![
                (0..5, rgba(0x3366ff80).into()),
                (6..11, rgb(0xffcc00).into()),
            ],
        )]);
        let plain_geometry = draw(
            cx,
            blocks.clone(),
            HashMap::new(),
            Default::default(),
            px(300.),
        );
        let decorated = draw(cx, blocks, decorations, Default::default(), px(300.));
        assert_eq!(
            decorated[&1].bounds().size,
            plain_geometry[&1].bounds().size,
            "decorations paint behind the text and never change layout"
        );
    }

    #[test]
    fn collect_leaf_geometry_nests_and_restores_on_panic() {
        let seen = Cell::new(0);
        let (inner_ran, geometries) = collect_leaf_geometry(HashMap::new(), || {
            assert!(editor_seam::current().is_some());
            let outer = editor_seam::current().unwrap();
            collect_leaf_geometry(HashMap::new(), || seen.set(seen.get() + 1));
            let after = editor_seam::current().unwrap();
            assert!(Rc::ptr_eq(&outer, &after), "the outer scope is restored");
            true
        });
        assert!(inner_ran && geometries.is_empty() && seen.get() == 1);
        assert!(editor_seam::current().is_none());

        let result = std::panic::catch_unwind(|| {
            collect_leaf_geometry(HashMap::new(), || panic!("boom"));
        });
        assert!(result.is_err());
        assert!(editor_seam::current().is_none(), "cleared after a panic");
    }

    #[test]
    fn parse_markdown_reads_what_an_editor_round_trips() {
        let source = "```rust title=\"x\"\nfn main() {}\n```\n\n\
                      | a | b | c | d |\n|---|:--|:-:|--:|\n| 1 | 2 | 3 | 4 |\n\n\
                      [ref]: https://example.com \"Title\"\n";
        let blocks = parse_markdown(source, Default::default()).unwrap();
        let code = blocks
            .iter()
            .find_map(|block| match block {
                BlockNode::CodeBlock(code) => Some(code),
                _ => None,
            })
            .expect("code block");
        assert_eq!(code.lang().as_deref(), Some("rust"));
        assert_eq!(code.meta().as_deref(), Some("title=\"x\""));
        let table = blocks
            .iter()
            .find_map(|block| match block {
                BlockNode::Table(table) => Some(table),
                _ => None,
            })
            .expect("table");
        assert_eq!(
            table.column_aligns(),
            [
                ColumnAlign::None,
                ColumnAlign::Left,
                ColumnAlign::Center,
                ColumnAlign::Right
            ]
        );
        let cells: Vec<Vec<String>> = table
            .rows()
            .map(|row| {
                row.map(|cell| cell.children().iter().map(|c| c.text()).collect())
                    .collect()
            })
            .collect();
        assert_eq!(cells, [["a", "b", "c", "d"], ["1", "2", "3", "4"]]);
        assert!(blocks.iter().any(|block| matches!(
            block,
            BlockNode::Definition { identifier, .. } if identifier.as_ref() == "ref"
        )));
        assert!(
            blocks
                .iter()
                .all(|block| block.span().is_some() || matches!(block, BlockNode::Unknown))
        );
    }

    #[test]
    fn parse_markdown_matches_text_view_parse() {
        let source = "# Title\n\nSome **bold** and `code` and ==mark==.\n\n- one\n- two\n";
        let mut cx = NodeContext::default();
        let expected = markdown::parse(source, &mut cx).unwrap();
        let blocks = parse_markdown(source, Default::default()).unwrap();
        assert_eq!(blocks, *expected.blocks);
    }

    #[test]
    fn table_and_paragraph_builders_read_back() {
        let mut paragraph = Paragraph::default();
        paragraph.push(InlineNode::new("hi").marks(vec![(0..2, TextMark::default().bold())]));
        assert_eq!(paragraph.children()[0].text(), "hi");
        assert!(paragraph.children()[0].mark_runs()[0].1.bold);
        assert!(paragraph.children()[0].as_image().is_none());
        assert!(paragraph.children()[0].as_custom().is_none());
        let table = Table::new(vec![vec![paragraph]], vec![ColumnAlign::Center]);
        assert_eq!(table.rows().count(), 1);
        assert_eq!(table.column_aligns(), [ColumnAlign::Center]);
    }
}
