use std::sync::Arc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Bounds, ClickEvent, ContentMask, Element, ElementId, Entity, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement, IntoElement, LayoutId,
    MouseButton, ParentElement, Pixels, Refineable as _, SharedString, StyleRefinement, Styled,
    Window, div, point, px,
};

use crate::StyledExt;
use crate::scroll::ScrollableElement;
use crate::text::TextViewFormat;
use crate::text::markdown_ext::{MarkdownExtensions, MarkdownNode, MarkdownPlugin};
use crate::text::node::{CodeBlock, TableData};
use crate::text::state::{LineSpan, SelectionFormat, TextViewState};
use crate::{global_state::UiGlobalState, text::TextViewStyle};

/// Type for code block actions generator function.
pub(crate) type CodeBlockActionsFn =
    dyn Fn(&CodeBlock, &mut Window, &mut App) -> AnyElement + Send + Sync;

/// Type for the table actions generator function.
pub(crate) type TableActionsFn =
    dyn Fn(&TableData, &mut Window, &mut App) -> AnyElement + Send + Sync;

pub(crate) type LinkClickHandlerFn =
    dyn Fn(&SharedString, &ClickEvent, &mut Window, &mut App) + Send + Sync;

pub(crate) fn handle_link_click(
    handler: &Option<Arc<LinkClickHandlerFn>>,
    url: SharedString,
    event: ClickEvent,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(handler) = handler {
        handler(&url, &event, window, cx);
    } else if match &event {
        ClickEvent::Mouse(click) => {
            matches!(click.up.button, MouseButton::Left | MouseButton::Middle)
        }
        ClickEvent::Keyboard(_) => true,
        ClickEvent::Touch(click) => !click.long_press,
    } {
        cx.open_url(&url);
    }
}

/// A text view that can render Markdown or HTML.
///
/// ## Goals
///
/// - Provide a rich text rendering component for such as Markdown or HTML,
/// used to display rich text in GPUI application (e.g., Help messages, Release notes)
/// - Support Markdown GFM and HTML (Simple HTML like Safari Reader Mode) for showing most common used markups.
/// - Support Heading, Paragraph, Bold, Italic, StrikeThrough, Code, Link, Image, Blockquote, List, Table, HorizontalRule, CodeBlock ...
///
/// ## Not Goals
///
/// - Customization of the complex style (some simple styles will be supported)
/// - As a Markdown editor or viewer (If you want to like this, you must fork your version).
/// - As a HTML viewer, we not support CSS, we only support basic HTML tags for used to as a content reader.
///
/// See also [`MarkdownElement`], [`HtmlElement`]
#[derive(Clone)]
pub struct TextView {
    id: ElementId,
    format: Option<TextViewFormat>,
    text: Option<SharedString>,
    pub(crate) state: Option<Entity<TextViewState>>,
    text_view_style: TextViewStyle,
    style: StyleRefinement,
    selectable: bool,
    selection_format: SelectionFormat,
    scrollable: bool,
    max_lines: Option<usize>,
    code_block_actions: Option<Arc<CodeBlockActionsFn>>,
    table_actions: Option<Arc<TableActionsFn>>,
    link_click_handler: Option<Arc<LinkClickHandlerFn>>,
    markdown_extensions: Arc<MarkdownExtensions>,
}

/// A plugin that can configure a [`TextView`].
pub trait TextViewPlugin {
    fn setup(self, text_view: TextView) -> TextView;
}

impl<P> TextViewPlugin for P
where
    P: MarkdownPlugin,
{
    fn setup(self, mut text_view: TextView) -> TextView {
        let extensions = Arc::make_mut(&mut text_view.markdown_extensions);
        let current = std::mem::take(extensions);
        *extensions = current.plugin(self);
        text_view
    }
}

impl Styled for TextView {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl TextView {
    /// Create new TextView with managed state.
    pub fn new(state: &Entity<TextViewState>) -> Self {
        Self {
            id: ElementId::Name(state.entity_id().to_string().into()),
            state: Some(state.clone()),
            format: None,
            text: None,
            text_view_style: TextViewStyle::default(),
            style: StyleRefinement::default(),
            selectable: false,
            selection_format: SelectionFormat::default(),
            scrollable: false,
            max_lines: None,
            code_block_actions: None,
            table_actions: None,
            link_click_handler: None,
            markdown_extensions: Arc::default(),
        }
    }

    /// Create a new markdown text view.
    pub fn markdown(id: impl Into<ElementId>, markdown: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            format: Some(TextViewFormat::Markdown),
            text: Some(markdown.into()),
            text_view_style: TextViewStyle::default(),
            style: StyleRefinement::default(),
            state: None,
            selectable: false,
            selection_format: SelectionFormat::default(),
            scrollable: false,
            max_lines: None,
            code_block_actions: None,
            table_actions: None,
            link_click_handler: None,
            markdown_extensions: Arc::default(),
        }
    }

    /// Create a new html text view.
    pub fn html(id: impl Into<ElementId>, html: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            format: Some(TextViewFormat::Html),
            text: Some(html.into()),
            text_view_style: TextViewStyle::default(),
            style: StyleRefinement::default(),
            state: None,
            selectable: false,
            selection_format: SelectionFormat::default(),
            scrollable: false,
            max_lines: None,
            code_block_actions: None,
            table_actions: None,
            link_click_handler: None,
            markdown_extensions: Arc::default(),
        }
    }

    /// Set [`TextViewStyle`].
    pub fn style(mut self, style: TextViewStyle) -> Self {
        self.text_view_style = style;
        self
    }

    /// Set the text view to be selectable, default is false.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Set the [`SelectionFormat`], default is [`SelectionFormat::Plain`].
    ///
    /// With [`SelectionFormat::Source`], selecting inside `**bold**` yields
    /// `**bold**` (the Markdown source) rather than `bold`.
    pub fn selection_format(mut self, selection_format: SelectionFormat) -> Self {
        self.selection_format = selection_format;
        self
    }

    /// Set the text view to be scrollable, default is false.
    ///
    /// ## If true for `scrollable`
    ///
    /// The `scrollable` mode used for large content,
    /// will show scrollbar, but requires the parent to have a fixed height,
    /// and use [`gpui::list`] to render the content in a virtualized way.
    ///
    /// ## If false to fit content
    ///
    /// The TextView will expand to fit all content, no scrollbar.
    /// This mode is suitable for small content, such as a few lines of text, a label, etc.
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    /// Clamp the rendered content to at most `n` lines of body text.
    ///
    /// The view's height is capped at `n` × the base line height, and a line
    /// of glyphs is never cut in half: a line that would straddle the bottom
    /// of the box is left out whole, across paragraphs, lists, headings, code
    /// blocks and tables. Nothing is shown with less than a line of itself to
    /// show, so the border and padding a table row leads with never strands at
    /// the bottom; whatever has more than that is cut on the box edge and keeps
    /// the part that fits, rather than disappearing and leaving blank space
    /// behind.
    ///
    /// Check [`TextViewState::is_clamped`] (which answers for the frame that
    /// was last painted) to decide whether to show an "expand" affordance.
    ///
    /// `n` counts lines of body text, so paragraph spacing and taller lines
    /// mean fewer of them fit inside the capped height. A line taller than the
    /// whole budget keeps the part that fits rather than emptying the box.
    /// Ignored when [`Self::scrollable`] is set.
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = Some(max_lines);
        self
    }

    /// Set custom block actions for code blocks.
    ///
    /// The closure receives the [`CodeBlock`],
    /// and returns an element to display.
    pub fn code_block_actions<F, E>(mut self, f: F) -> Self
    where
        F: Fn(&CodeBlock, &mut Window, &mut App) -> E + Send + Sync + 'static,
        E: IntoElement,
    {
        self.code_block_actions = Some(Arc::new(move |code_block, window, cx| {
            f(&code_block, window, cx).into_any_element()
        }));
        self
    }

    /// Set custom actions to be rendered below each Markdown table.
    ///
    /// The closure receives the [`TableData`],
    /// and returns an element to display.
    pub fn table_actions<F, E>(mut self, f: F) -> Self
    where
        F: Fn(&TableData, &mut Window, &mut App) -> E + Send + Sync + 'static,
        E: IntoElement,
    {
        self.table_actions = Some(Arc::new(move |table, window, cx| {
            f(table, window, cx).into_any_element()
        }));
        self
    }

    /// Handle pointer events on rendered links.
    ///
    /// The handler receives the resolved URL and the original GPUI click event.
    /// Without a handler, links open through App::open_url.
    pub fn on_link_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&SharedString, &ClickEvent, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.link_click_handler = Some(Arc::new(handler));
        self
    }

    /// Replace the Markdown extension registry.
    pub fn markdown_extensions(mut self, extensions: MarkdownExtensions) -> Self {
        self.markdown_extensions = Arc::new(extensions);
        self
    }

    /// Enable MDX JSX/expression parsing.
    ///
    /// This disables raw HTML parsing because `markdown-rs` gives HTML
    /// priority over MDX when both are enabled.
    pub fn markdown_mdx(mut self) -> Self {
        let extensions = Arc::make_mut(&mut self.markdown_extensions);
        *extensions = extensions.clone().mdx();
        self
    }

    /// Register a custom block-level Markdown parser.
    ///
    /// The parser runs during Markdown AST conversion and must be independent
    /// of [`Window`] / [`App`]. Store any parsed data in [`MarkdownNode`] and
    /// render it later with [`Self::markdown_block_renderer`].
    pub fn markdown_block_parser<F>(mut self, parser: F) -> Self
    where
        F: for<'a> Fn(
                &markdown::mdast::Node,
                &crate::text::MarkdownParseContext<'a>,
            ) -> Option<MarkdownNode>
            + Send
            + Sync
            + 'static,
    {
        Arc::make_mut(&mut self.markdown_extensions).push_block_parser(parser);
        self
    }

    /// Register a renderer for a custom block-level Markdown node name.
    pub fn markdown_block_renderer<F, E>(
        mut self,
        name: impl Into<SharedString>,
        renderer: F,
    ) -> Self
    where
        F: Fn(&MarkdownNode, &mut Window, &mut App) -> E + Send + Sync + 'static,
        E: IntoElement,
    {
        Arc::make_mut(&mut self.markdown_extensions).push_block_renderer(name, renderer);
        self
    }

    /// Apply a reusable text view plugin.
    pub fn plugin<P>(self, plugin: P) -> Self
    where
        P: TextViewPlugin,
    {
        plugin.setup(self)
    }
}

impl IntoElement for TextView {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

pub struct TextViewLayoutState {
    state: Entity<TextViewState>,
    element: AnyElement,
}

pub struct TextViewPrepaintState {
    hitbox: Hitbox,
    /// Where paint has to pull the `max_lines` clip up to, because a glyph line
    /// straddles the bottom of the box. `None` leaves the clip at the box edge,
    /// where the container's hidden overflow already applies it.
    clip_bottom: Option<Pixels>,
}

/// Absorbs sub-pixel layout jitter: a line ending within a pixel of the box
/// bottom counts as fitting inside it.
const CLIP_EPSILON: Pixels = px(1.);

/// The bottom of the last whole line at or above `y`, with the height of a line
/// where it sits.
fn last_line_bottom_above(spans: &[LineSpan], y: Pixels) -> Option<(Pixels, Pixels)> {
    let mut last: Option<(Pixels, Pixels)> = None;
    let mut keep = |bottom: Pixels, line_height: Pixels| {
        if bottom <= y + CLIP_EPSILON && last.is_none_or(|(last, _)| bottom > last) {
            last = Some((bottom, line_height));
        }
    };

    for span in spans {
        if span.line_height <= px(0.) {
            continue;
        }
        let mut bottom = span.top + span.line_height;
        while bottom <= span.bottom + CLIP_EPSILON {
            keep(bottom, span.line_height);
            bottom += span.line_height;
        }
        // The span's own bottom covers a last line taller than the rest.
        keep(span.bottom, span.line_height);
    }

    last
}

/// Where to clip, given the lines a descendant `Inline` reported. `None` leaves
/// the clip on the box edge.
///
/// Two things are never shown: half a line of glyphs, and anything with less
/// than a line of itself to show. A line straddling `box_bottom` is left out
/// whole, and so is the strip between it and the line before — the border and
/// padding a table row leads with reads as a rendering fault rather than as a
/// row. Whatever has more than a line to show is cut on the edge and keeps the
/// part that fits, so the box holds no blank space it could have filled.
fn line_safe_clip_bottom(
    spans: &[LineSpan],
    box_bottom: Pixels,
    content_bottom: Pixels,
) -> Option<Pixels> {
    let mut clip = box_bottom;

    for span in spans {
        if span.line_height <= px(0.)
            || span.top >= box_bottom
            || span.bottom <= box_bottom + CLIP_EPSILON
        {
            continue;
        }
        let whole_lines = ((box_bottom - span.top) / span.line_height).floor();
        let line_top = span.top + span.line_height * whole_lines;
        // A line starting on the box edge is not straddling it.
        if line_top < box_bottom - CLIP_EPSILON {
            clip = clip.min(line_top);
        }
    }

    let Some((last_line_bottom, line_height)) = last_line_bottom_above(spans, clip) else {
        // Leaving the straddling line out would leave nothing at all — a first
        // line taller than the whole budget, a heading in a one-line box. It
        // keeps the part that fits instead, because an empty clamp reads as
        // broken where a cut one reads as more to come.
        return None;
    };

    // Snap away a scrap. Only content that continues past the box can leave
    // one: the space under the last line of a document that fits is the box's
    // own, not a piece of something below.
    if content_bottom > box_bottom + CLIP_EPSILON {
        let strip = clip - last_line_bottom;
        if strip > CLIP_EPSILON && strip < line_height {
            clip = last_line_bottom;
        }
    }

    (clip < box_bottom - CLIP_EPSILON).then_some(clip)
}

impl Element for TextView {
    type RequestLayoutState = TextViewLayoutState;
    type PrepaintState = TextViewPrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
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
    ) -> (LayoutId, Self::RequestLayoutState) {
        let state = if let Some(state) = self.state.clone() {
            state
        } else {
            let default_format = self.format.unwrap_or(TextViewFormat::Markdown);
            let default_text = self.text.clone().unwrap_or_default();

            let state = window.use_keyed_state(
                SharedString::from(format!("{}/state", self.id)),
                cx,
                move |_, cx| {
                    if default_format == TextViewFormat::Markdown {
                        TextViewState::markdown(default_text.as_str(), cx)
                    } else {
                        TextViewState::html(default_text.as_str(), cx)
                    }
                },
            );
            self.state = Some(state.clone());
            state
        };

        // `max_lines` needs the whole document laid out to snap the clip to a
        // whole line, so it only applies to the fit-content mode.
        let max_lines = self.max_lines.filter(|_| !self.scrollable);

        state.update(cx, |state, cx| {
            state.code_block_actions = self.code_block_actions.clone();
            state.table_actions = self.table_actions.clone();
            state.link_click_handler = self.link_click_handler.clone();
            state.set_markdown_extensions(self.markdown_extensions.clone(), cx);
            state.selectable = self.selectable;
            state.selection_format = self.selection_format;
            state.scrollable = self.scrollable;
            state.max_lines = max_lines;
            if state.text_view_style != self.text_view_style {
                state.selection_revision = state.selection_revision.wrapping_add(1);
            }
            state.text_view_style = self.text_view_style.clone();

            if let Some(text) = self.text.clone() {
                state.set_text(text.as_str(), cx);
            }
        });

        let focus_handle = state.read(cx).focus_handle.clone();
        let list_state = state.read(cx).list_state.clone();

        // Cap the box at `n` body-text lines (the effective text style may be
        // refined by this view's own style, e.g. `.text_sm()`); hidden
        // overflow also clips descendant hitboxes to the box during prepaint.
        let max_lines_cap = max_lines.map(|max_lines| {
            let mut text_style = window.text_style();
            text_style.refine(&self.style.text);
            text_style.line_height_in_pixels(window.rem_size()) * max_lines as f32
        });

        let mut el = div()
            .key_context("TextView")
            .track_focus(&focus_handle)
            .when(self.scrollable, |this| {
                this.size_full().vertical_scrollbar(&list_state)
            })
            .when_some(max_lines_cap, |this, cap| this.max_h(cap).overflow_hidden())
            .relative()
            .on_action(move |_: &crate::input::Copy, window, cx| {
                let text = gpui_base::TextSelection::selected_text(window, cx)
                    .trim()
                    .to_string();
                if text.is_empty() {
                    cx.propagate();
                    return;
                }
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            })
            .on_action(window.listener_for(&state, TextViewState::on_action_select_all))
            .child(state.clone())
            .refine_style(&self.style)
            .into_any_element();
        let layout_id = el.request_layout(window, cx);
        (layout_id, TextViewLayoutState { state, element: el })
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let state = request_layout.state.clone();
        let max_lines_active = state.read(cx).max_lines.is_some();
        if max_lines_active {
            if let Ok(mut line_spans) = state.read(cx).line_spans.lock() {
                line_spans.clear();
            }
            // Descendant `Inline`s report their line spans through the state
            // stack during prepaint (in addition to the paint-time push below).
            UiGlobalState::global_mut(cx)
                .text_view_state_stack
                .push(state.clone());
        }
        request_layout.element.prepaint(window, cx);
        if max_lines_active {
            UiGlobalState::global_mut(cx).text_view_state_stack.pop();
        }

        let mut clip_bottom = None;
        if max_lines_active {
            let (line_spans, content_bottom) = {
                let state = state.read(cx);
                (
                    state
                        .line_spans
                        .lock()
                        .map(|spans| spans.clone())
                        .unwrap_or_default(),
                    state.bounds().bottom(),
                )
            };
            // The content keeps its natural height inside the capped box, so
            // this sees everything the box cannot show — including a tall image
            // that reports no lines of its own.
            let clipped = content_bottom > bounds.bottom() + px(1.);
            // Notify on change so observers (e.g. an "expand" button gated on
            // `is_clamped`) re-render once the flag flips.
            if state.read(cx).clamped != clipped {
                state.update(cx, |state, cx| {
                    state.clamped = clipped;
                    cx.notify();
                });
            }
            if clipped {
                clip_bottom = line_safe_clip_bottom(&line_spans, bounds.bottom(), content_bottom);
            }
        }

        TextViewPrepaintState {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
            clip_bottom,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = &request_layout.state;
        if self.selectable {
            state.update(cx, |state, _| state.selection_adapter.begin_frame());
        }

        UiGlobalState::global_mut(cx)
            .text_view_state_stack
            .push(state.clone());
        if let Some(clip_bottom) = prepaint.clip_bottom {
            // Snap the `max_lines` clip to the last whole line that fits, so a
            // line of glyphs is never cut in half.
            let mask = ContentMask {
                bounds: Bounds::from_corners(bounds.origin, point(bounds.right(), clip_bottom)),
            };
            window.with_content_mask(Some(mask), |window| {
                request_layout.element.paint(window, cx);
            });
        } else {
            request_layout.element.paint(window, cx);
        }
        UiGlobalState::global_mut(cx).text_view_state_stack.pop();

        if self.selectable {
            let (adapter, scroll_offset, content_bounds) = {
                let state = state.read(cx);
                (
                    state.selection_adapter.clone(),
                    state.scroll_offset(),
                    state.bounds(),
                )
            };
            let document_order = UiGlobalState::global_mut(cx).next_selection_document_order();
            adapter.register(
                prepaint.hitbox.clone(),
                content_bounds,
                scroll_offset,
                document_order,
                window,
                cx,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TextView, TextViewPlugin};
    use crate::text::{TableData, TextViewState, TextViewStyle};
    use gpui::{
        AppContext as _, Bounds, ClickEvent, Context, Entity, InteractiveElement as _, IntoElement,
        Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Overflow, ParentElement as _, Pixels,
        Render, SharedString, StyleRefinement, Styled as _, TestAppContext, VisualTestContext,
        Window, div, point, px,
    };

    struct TextViewTestRoot {
        text_view: Entity<TextViewState>,
    }

    struct DummyTextViewPlugin;

    impl TextViewPlugin for DummyTextViewPlugin {
        fn setup(self, mut text_view: TextView) -> TextView {
            text_view.selectable = true;
            text_view
        }
    }

    impl TextViewTestRoot {
        fn new(text: &str, cx: &mut Context<Self>) -> Self {
            let text = text.to_string();
            let text_view = cx.new(|cx| TextViewState::markdown(&text, cx));
            Self { text_view }
        }
    }

    impl Render for TextViewTestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(160.))
                .child(
                    div()
                        .h(px(24.))
                        .overflow_hidden()
                        .child(TextView::new(&self.text_view).selectable(true)),
                )
                .child(div().h(px(40.)).child("footer"))
        }
    }

    struct InlineImageTextViewTestRoot {
        text_view: Entity<TextViewState>,
    }

    impl InlineImageTextViewTestRoot {
        fn new(cx: &mut Context<Self>) -> Self {
            let text_view = cx.new(|cx| {
                TextViewState::markdown(
                    "Build Status ![inline image](https://example.com/image.svg) after",
                    cx,
                )
            });
            Self { text_view }
        }
    }

    impl Render for InlineImageTextViewTestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(420.))
                .child(TextView::new(&self.text_view).selectable(true))
        }
    }

    #[gpui::test]
    fn inline_image_keeps_surrounding_text_on_same_line(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|cx| InlineImageTextViewTestRoot::new(cx));
            crate::Root::new(content, window, cx)
        });
        let content = root.read_with(cx, |root, _| {
            root.view()
                .clone()
                .downcast::<InlineImageTextViewTestRoot>()
                .unwrap()
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let inline_bounds = content.read_with(cx, |content, cx| {
            content.text_view.read(cx).selection_adapter.text_bounds()
        });

        assert_eq!(inline_bounds.len(), 2);
        assert_eq!(
            inline_bounds[0].top(),
            inline_bounds[1].top(),
            "text before and after an inline image should share a rendered line"
        );
        assert!(
            inline_bounds[1].left() - inline_bounds[0].right() > px(8.),
            "inline image should reserve horizontal space in the text layout"
        );
        assert!(
            inline_bounds[1].left() - inline_bounds[0].right() < px(40.),
            "unloaded inline image fallback should stay generic and compact"
        );
    }

    #[gpui::test]
    fn inline_html_image_after_newline_does_not_panic(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, cx| {
            TextViewTestRoot::new(
                "Hi\n[<img src=\"https://example.com/image.svg\">](https://google.com/)",
                cx,
            )
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
    }

    #[gpui::test]
    fn list_item_renders_fenced_code_block_at_document_width(cx: &mut TestAppContext) {
        struct ListItemBlockRoot;

        impl Render for ListItemBlockRoot {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                div().w(px(840.)).h(px(400.)).child(
                    crate::h_resizable("markdown-width-test")
                        .child(crate::resizable_panel().child(div()))
                        .child(crate::resizable_panel().child(
                            TextView::markdown(
                                "list-with-code",
                                "1. List item\n   ```rust\n   nested code\n   ```\n\n```rust\ntop-level code\n```",
                            )
                            .code_block_actions(|code_block, _, _| {
                                let selector = if code_block.code().contains("nested") {
                                    "nested-code-action"
                                } else {
                                    "top-level-code-action"
                                };
                                div()
                                    .debug_selector(move || selector.into())
                                    .child("Copy")
                            })
                            .scrollable(true)
                            .p_5()
                            .flex_none(),
                        )),
                )
            }
        }

        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| ListItemBlockRoot);
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let nested_action = cx.debug_bounds("nested-code-action").unwrap();
        let top_level_action = cx.debug_bounds("top-level-code-action").unwrap();
        assert!(
            top_level_action.right() - nested_action.right() < px(32.),
            "nested code block should fill the list item's available width"
        );
    }

    /// Draw a Markdown table with a `table_actions` hook installed, and return
    /// the painted bounds of the actions element plus the data it received.
    /// `scroll` opts into the horizontally scrollable table layout.
    fn draw_table_with_actions(
        cx: &mut TestAppContext,
        scroll: bool,
    ) -> (Bounds<Pixels>, TableData) {
        use std::sync::{Arc, Mutex};

        struct TableRoot {
            scroll: bool,
            captured: Arc<Mutex<Vec<TableData>>>,
        }

        impl Render for TableRoot {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                let captured = self.captured.clone();
                let mut table_style = StyleRefinement::default();
                if self.scroll {
                    table_style.overflow.x = Some(Overflow::Scroll);
                }

                div().w(px(320.)).child(
                    TextView::markdown(
                        "table-actions",
                        "| Name | Age |\n|:--|--:|\n| Alice | 30 |\n| Bob | 41 |",
                    )
                    .style(TextViewStyle::default().table(table_style))
                    .table_actions(move |table, _, _| {
                        if let Ok(mut captured) = captured.lock() {
                            captured.push(table.clone());
                        }
                        div().debug_selector(|| "table-action".into()).child("Copy")
                    }),
                )
            }
        }

        cx.update(crate::init);
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let captured = captured.clone();
            move |_, _| TableRoot { scroll, captured }
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let bounds = cx
            .debug_bounds("table-action")
            .expect("table actions should be painted");
        let data = captured
            .lock()
            .expect("captured table data")
            .last()
            .cloned()
            .expect("table actions hook should receive the table");

        (bounds, data)
    }

    #[gpui::test]
    fn table_actions_render_below_the_table(cx: &mut TestAppContext) {
        for scroll in [false, true] {
            let (bounds, data) = draw_table_with_actions(cx, scroll);

            // Header plus two data rows are painted above the actions row.
            assert!(
                bounds.top() > px(40.),
                "actions should sit below the table (scroll: {scroll}), got {:?}",
                bounds.top()
            );
            assert_eq!(data.headers, vec!["Name", "Age"]);
            assert_eq!(data.rows, vec![vec!["Alice", "30"], vec!["Bob", "41"]]);
            assert_eq!(
                data.markdown,
                "| Name | Age |\n| :-- | --: |\n| Alice | 30 |\n| Bob | 41 |"
            );
            assert_eq!(data.span, Some(0..52));
        }
    }

    #[test]
    fn plugin_accepts_text_view_plugins_beyond_markdown() {
        let view = TextView::markdown("plugin-test", "").plugin(DummyTextViewPlugin);

        assert!(view.selectable);
    }

    #[gpui::test]
    fn clipped_markdown_link_does_not_open(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, cx| {
            TextViewTestRoot::new("visible\n\n[hidden](https://example.com)", cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.simulate_click(point(px(10.), px(34.)), Modifiers::default());

        assert_eq!(cx.opened_url(), None);
    }

    struct MaxLinesTestRoot {
        text_view: Entity<TextViewState>,
        max_lines: usize,
    }

    impl MaxLinesTestRoot {
        fn new(text: &str, max_lines: usize, cx: &mut Context<Self>) -> Self {
            let text_view = cx.new(|cx| TextViewState::markdown(text, cx));
            Self {
                text_view,
                max_lines,
            }
        }
    }

    impl Render for MaxLinesTestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(200.))
                .child(TextView::new(&self.text_view).max_lines(self.max_lines))
        }
    }

    #[test]
    fn the_clip_only_moves_for_a_straddling_glyph_line() {
        use super::line_safe_clip_bottom;
        use crate::text::state::LineSpan;

        let spans = [
            // Lines end at 20 / 40 / 60.
            LineSpan {
                top: px(0.),
                bottom: px(60.),
                line_height: px(20.),
            },
            // A second block after an 8px gap; lines end at 88 / 108 / 128.
            LineSpan {
                top: px(68.),
                bottom: px(128.),
                line_height: px(20.),
            },
        ];

        // Content continues well past the box in every case but the last.
        let below = px(400.);

        // A box ending inside the line 88..108 leaves that line out whole.
        assert_eq!(
            line_safe_clip_bottom(&spans, px(100.), below),
            Some(px(88.))
        );

        // A box ending on a line boundary has nothing to pull the clip up for.
        assert_eq!(line_safe_clip_bottom(&spans, px(88.), below), None);

        // A strip below the last line shorter than a line — the border and
        // padding a block leads with — is not worth showing.
        assert_eq!(line_safe_clip_bottom(&spans, px(64.), below), Some(px(60.)));

        // One taller than a line is: whatever crosses the edge keeps the part
        // that fits rather than leaving the box half empty.
        let one_block = [LineSpan {
            top: px(0.),
            bottom: px(60.),
            line_height: px(20.),
        }];
        assert_eq!(line_safe_clip_bottom(&one_block, px(200.), below), None);

        // Nothing crosses the edge at all: the space under the last line is
        // the box's own, not a scrap of something below.
        assert_eq!(line_safe_clip_bottom(&spans, px(130.), px(128.)), None);
    }

    #[test]
    fn a_line_taller_than_the_budget_keeps_the_part_that_fits() {
        use super::line_safe_clip_bottom;
        use crate::text::state::LineSpan;

        // A heading line of 28px, in a box capped at one 26px body line.
        let heading = [LineSpan {
            top: px(70.),
            bottom: px(98.),
            line_height: px(28.),
        }];

        assert_eq!(line_safe_clip_bottom(&heading, px(96.), px(400.)), None);
    }

    #[test]
    fn the_clip_does_not_stop_on_a_row_of_border_and_padding() {
        use super::line_safe_clip_bottom;
        use crate::text::state::LineSpan;

        // Two table rows, each one line of text, 9px of border and padding
        // between them.
        let rows = [
            LineSpan {
                top: px(100.),
                bottom: px(126.),
                line_height: px(26.),
            },
            LineSpan {
                top: px(135.),
                bottom: px(161.),
                line_height: px(26.),
            },
        ];

        // Leaving out the second row's text would strand the 9px it leads
        // with, so the clip goes back to the row above it.
        assert_eq!(
            line_safe_clip_bottom(&rows, px(148.), px(400.)),
            Some(px(126.))
        );
    }

    /// A clamped view nested the way an application nests one: inside a card,
    /// inside a region that fills a window of a known height. The height an
    /// ancestor hands down must not reach the clamped content and hide the
    /// overflow the clamp measures — with the content stretched to the capped
    /// box, nothing looks clipped and lines get cut in half.
    struct ClampedPageRoot {
        text_view: Entity<TextViewState>,
        max_lines: usize,
    }

    impl Render for ClampedPageRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            use crate::scroll::ScrollableElement as _;
            use crate::{h_flex, v_flex};

            v_flex()
                .size_full()
                .p_4()
                .gap_4()
                .child(h_flex().max_w(px(480.)).gap_3().child("header"))
                .child(
                    v_flex()
                        .flex_1()
                        .min_h_0()
                        .gap_4()
                        .child(
                            v_flex()
                                .max_w(px(480.))
                                .p_3()
                                .gap_2()
                                .child(TextView::new(&self.text_view).max_lines(self.max_lines)),
                        )
                        .overflow_y_scrollbar(),
                )
        }
    }

    #[gpui::test]
    fn max_lines_measures_overflow_inside_a_sized_page(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|_, cx| {
            let text_view = cx.new(|cx| {
                TextViewState::markdown(
                    "first\n\nsecond\n\nthird\n\nfourth\n\nfifth\n\nsixth\n\nseventh",
                    cx,
                )
            });
            ClampedPageRoot {
                text_view,
                max_lines: 3,
            }
        });
        let cx: &mut VisualTestContext = cx;

        assert!(root.read_with(cx, |root, cx| root.text_view.read(cx).is_clamped()));
    }

    #[gpui::test]
    fn max_lines_clamps_overflowing_content(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|_, cx| {
            MaxLinesTestRoot::new(
                "first\n\nsecond\n\nthird\n\nfourth\n\nfifth\n\nsixth",
                2,
                cx,
            )
        });
        let cx: &mut VisualTestContext = cx;

        assert!(root.read_with(cx, |root, cx| root.text_view.read(cx).is_clamped()));
    }

    #[gpui::test]
    fn max_lines_leaves_short_content_unclamped(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|_, cx| MaxLinesTestRoot::new("only line", 3, cx));
        let cx: &mut VisualTestContext = cx;

        assert!(!root.read_with(cx, |root, cx| root.text_view.read(cx).is_clamped()));
    }

    #[gpui::test]
    fn max_lines_disables_links_hidden_by_the_clamp(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, cx| {
            MaxLinesTestRoot::new(
                "first\n\nsecond\n\nthird\n\n[hidden](https://example.com)",
                2,
                cx,
            )
        });
        let cx: &mut VisualTestContext = cx;

        // Click far below the clamped box, where the link would sit unclamped.
        cx.simulate_click(point(px(10.), px(150.)), Modifiers::default());

        assert_eq!(cx.opened_url(), None);
    }

    #[gpui::test]
    fn markdown_link_opens_url_without_handler(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) =
            cx.add_window_view(|_, cx| TextViewTestRoot::new("[example](https://example.com)", cx));
        let cx: &mut VisualTestContext = cx;

        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());

        assert_eq!(cx.opened_url(), Some("https://example.com".to_string()));
    }

    #[gpui::test]
    fn right_click_does_not_open_url_without_handler(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) =
            cx.add_window_view(|_, cx| TextViewTestRoot::new("[example](https://example.com)", cx));
        let cx: &mut VisualTestContext = cx;

        cx.simulate_mouse_down(
            point(px(10.), px(10.)),
            MouseButton::Right,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(10.), px(10.)),
            MouseButton::Right,
            Modifiers::default(),
        );

        assert_eq!(cx.opened_url(), None);
    }

    #[gpui::test]
    fn link_handler_receives_button_and_modifiers(cx: &mut TestAppContext) {
        use std::sync::{Arc, Mutex};

        struct LinkRoot {
            text_view: Entity<TextViewState>,
            clicks: Arc<Mutex<Vec<(SharedString, ClickEvent)>>>,
        }

        impl Render for LinkRoot {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                let clicks = self.clicks.clone();
                div()
                    .w(px(240.))
                    .child(
                        TextView::new(&self.text_view).on_link_click(move |url, event, _, _| {
                            clicks.lock().unwrap().push((url.clone(), event.clone()));
                        }),
                    )
            }
        }

        cx.update(crate::init);
        let clicks = Arc::new(Mutex::new(Vec::new()));
        let captured = clicks.clone();
        let (_, cx) = cx.add_window_view(move |_, cx| LinkRoot {
            text_view: cx.new(|cx| TextViewState::markdown("[example](https://example.com)", cx)),
            clicks,
        });
        let cx: &mut VisualTestContext = cx;

        let mut modifiers = Modifiers::default();
        modifiers.control = true;
        cx.simulate_click(point(px(10.), px(10.)), modifiers);
        cx.simulate_mouse_down(
            point(px(10.), px(10.)),
            MouseButton::Middle,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(10.), px(10.)),
            MouseButton::Middle,
            Modifiers::default(),
        );
        cx.simulate_mouse_down(
            point(px(10.), px(10.)),
            MouseButton::Right,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(10.), px(10.)),
            MouseButton::Right,
            Modifiers::default(),
        );

        let clicks = captured.lock().unwrap();
        assert_eq!(clicks.len(), 3);
        assert_eq!(clicks[0].0, "https://example.com");
        assert!(!clicks[0].1.is_right_click() && !clicks[0].1.is_middle_click());
        assert!(clicks[0].1.modifiers().control);
        assert!(clicks[1].1.is_middle_click());
        assert!(clicks[2].1.is_right_click());
        assert_eq!(cx.opened_url(), None);
    }

    #[gpui::test]
    fn linked_image_handler_receives_left_middle_and_right_clicks(cx: &mut TestAppContext) {
        use std::sync::{Arc, Mutex};

        struct LinkedImageRoot {
            text_view: Entity<TextViewState>,
            clicks: Arc<Mutex<Vec<(SharedString, ClickEvent)>>>,
        }

        impl Render for LinkedImageRoot {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                let clicks = self.clicks.clone();
                div().w(px(160.)).child(
                    TextView::new(&self.text_view)
                        .selectable(true)
                        .on_link_click(move |url, event, _, _| {
                            clicks.lock().unwrap().push((url.clone(), event.clone()));
                        }),
                )
            }
        }

        cx.update(crate::init);
        let clicks = Arc::new(Mutex::new(Vec::new()));
        let captured = clicks.clone();
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let content = cx.new(|cx| LinkedImageRoot {
                text_view: cx.new(|cx| {
                    TextViewState::markdown(
                        r#"Before [<img src="https://example.com/image.svg" width="32" height="32">](https://example.com/image-link) after."#,
                        cx,
                    )
                }),
                clicks,
            });
            crate::Root::new(content, window, cx)
        });
        let content = root.read_with(cx, |root, _| {
            root.view().clone().downcast::<LinkedImageRoot>().unwrap()
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let inline_bounds = content.read_with(cx, |content, cx| {
            content.text_view.read(cx).selection_adapter.text_bounds()
        });
        assert!(
            inline_bounds.len() >= 2,
            "linked image needs text bounds on both sides: {inline_bounds:?}"
        );
        assert!(
            inline_bounds[1].left() - inline_bounds[0].right() >= px(24.),
            "linked image did not reserve the expected click target: {inline_bounds:?}"
        );
        let position = point(
            inline_bounds[0].right() + (inline_bounds[1].left() - inline_bounds[0].right()) * 0.5,
            inline_bounds[0].top() + px(8.),
        );
        for button in [MouseButton::Left, MouseButton::Middle, MouseButton::Right] {
            cx.simulate_mouse_down(position, button, Modifiers::default());
            cx.simulate_mouse_up(position, button, Modifiers::default());
        }

        let clicks = captured.lock().unwrap();
        assert_eq!(clicks.len(), 3);
        assert!(
            clicks
                .iter()
                .all(|(url, _)| url == "https://example.com/image-link")
        );
        assert!(!clicks[0].1.is_right_click() && !clicks[0].1.is_middle_click());
        assert!(clicks[1].1.is_middle_click());
        assert!(clicks[2].1.is_right_click());
        assert_eq!(cx.opened_url(), None);
    }

    #[gpui::test]
    fn clipped_markdown_cannot_start_selection(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx
            .add_window_view(|_, cx| TextViewTestRoot::new("visible\n\nhidden selection text", cx));
        let cx: &mut VisualTestContext = cx;

        cx.simulate_mouse_down(
            point(px(10.), px(34.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(px(90.), px(34.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(90.), px(34.)),
            MouseButton::Left,
            Modifiers::default(),
        );

        let selected_text = view.read_with(cx, |root, cx| root.text_view.read(cx).selected_text());
        assert!(
            selected_text.is_empty(),
            "unexpected selection: {selected_text:?}"
        );
    }

    /// A tall selectable TextView clipped by a short `overflow_hidden` viewport,
    /// with a large blank footer below so a drag can extend the selection band
    /// past the bottom of the clip while still proxy-anchoring to the view.
    struct ClippedTallTextViewTestRoot {
        text_view: Entity<TextViewState>,
    }

    impl ClippedTallTextViewTestRoot {
        fn new(cx: &mut Context<Self>) -> Self {
            // Four separate blocks; only the first (and maybe part of the
            // second) fit inside the 40px clip. "charlie"/"delta" render well
            // below it.
            let text_view =
                cx.new(|cx| TextViewState::markdown("alpha\n\nbravo\n\ncharlie\n\ndelta", cx));
            Self { text_view }
        }
    }

    impl Render for ClippedTallTextViewTestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(200.))
                .child(
                    div()
                        .h(px(40.))
                        .overflow_hidden()
                        .child(TextView::new(&self.text_view).selectable(true)),
                )
                // A tall blank footer so a drag can reach a y below the clipped
                // text; a press there proxy-anchors to the TextView above.
                .child(div().h(px(160.)))
        }
    }

    /// Regression for copying a selection taller than the visible viewport.
    ///
    /// The selection band runs from visible text at the top down to a point
    /// far below the clip. Every glyph of the painted TextView is laid out even
    /// though the lower ones are clipped away, so the copied text must include
    /// the clipped-out "charlie"/"delta" — not just what is on screen. This
    /// guards against re-adding a `content_mask` gate in
    /// `Inline::layout_selections`.
    #[gpui::test]
    fn selection_band_beyond_clip_copies_offscreen_text(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(ClippedTallTextViewTestRoot::new);
            crate::Root::new(content, window, cx)
        });
        let content = view.read_with(cx, |root, _| {
            root.view()
                .clone()
                .downcast::<ClippedTallTextViewTestRoot>()
                .unwrap()
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        // Anchor on visible text near the top, then drag to a point well below
        // the 40px clip (into the blank footer) and to the far right so the
        // last line is fully covered.
        cx.simulate_mouse_down(
            point(px(2.), px(8.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.simulate_mouse_move(
            point(px(180.), px(150.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.simulate_mouse_up(
            point(px(180.), px(150.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let selected_text =
            content.read_with(cx, |root, cx| root.text_view.read(cx).selected_text());
        assert!(
            selected_text.contains("delta"),
            "clipped-out text was not copied: {selected_text:?}"
        );
        assert!(
            selected_text.contains("charlie"),
            "clipped-out text was not copied: {selected_text:?}"
        );
    }

    #[gpui::test]
    fn double_click_selects_word(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) =
            cx.add_window_view(|_, cx| TextViewTestRoot::new("quick select value", cx));

        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        let position = point(px(10.), px(16.));
        cx.simulate_event(MouseDownEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 2,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 2,
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let selected_text = view.read_with(cx, |root, cx| root.text_view.read(cx).selected_text());
        assert_eq!(selected_text.trim(), "quick");
    }

    #[gpui::test]
    fn triple_click_selects_paragraph(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) =
            cx.add_window_view(|_, cx| TextViewTestRoot::new("quick select value", cx));

        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let position = point(px(10.), px(10.));
        cx.simulate_event(MouseDownEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 3,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 3,
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let selected_text = view.read_with(cx, |root, cx| root.text_view.read(cx).selected_text());
        assert_eq!(selected_text.trim(), "quick select value");
    }

    // Regression: markdown `TextView` items inside an outer `gpui::list` with
    // `measure_all` must keep a stable total content height while scrolling.
    // Before synchronous full-replace parsing, off-screen markdown views were
    // first measured with empty content and the scrollbar thumb jittered as the
    // total height grew during scrolling.
    #[gpui::test]
    fn outer_list_content_total_stable_while_scrolling(cx: &mut TestAppContext) {
        use gpui::{ListAlignment, ListState, list};

        const ITEMS: &[&str] = &[
            "# Heading\n\nA paragraph long enough to wrap across several lines and produce a non-trivial height.",
            "Short.",
            "Paragraph A\n\nParagraph B\n\nParagraph C with more words to increase the height.",
            "## Subheading\n\n- One\n- Two\n- Three\n\nClosing paragraph.",
            "Only one line.",
            "**Bold**: medium length text with `code` mixed with regular words.",
            "1. First\n2. Second\n3. Third\n\nA short closing paragraph.",
            "A long message with enough words to wrap across multiple lines, create a taller item, and verify that off-screen measurement matches visible measurement.",
        ];
        let n = 40usize;

        struct ListRoot {
            state: ListState,
        }
        impl Render for ListRoot {
            fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                div().w(px(360.)).h(px(500.)).child(
                    list(self.state.clone(), |ix, _w, _cx| {
                        div()
                            .w_full()
                            .child(TextView::markdown(
                                ("md", ix as u64),
                                ITEMS[ix % ITEMS.len()],
                            ))
                            .into_any_element()
                    })
                    .size_full(),
                )
            }
        }

        cx.update(crate::init);
        let state = ListState::new(n, ListAlignment::Top, px(2048.)).measure_all();
        let probe = state.clone();
        let (_view, cx) = cx.add_window_view(|_w, _cx| ListRoot { state });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|w, cx| {
            let _ = w.draw(cx);
        });
        cx.run_until_parked();
        cx.update(|w, cx| {
            let _ = w.draw(cx);
        });

        let total = |p: &ListState| {
            f32::from(p.max_offset_for_scrollbar().y + p.viewport_bounds().size.height)
        };
        let mut totals = vec![total(&probe)];
        for _ in 0..20 {
            probe.scroll_by(px(150.));
            cx.update(|w, cx| {
                let _ = w.draw(cx);
            });
            cx.run_until_parked();
            totals.push(total(&probe));
        }
        let min = totals.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = totals.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        println!(
            "OUTER_LIST_PROBE min={min:.1} max={max:.1} delta={:.1}",
            max - min
        );
        assert!(
            (max - min) < 2.0,
            "list content total jittered while scrolling: min={min} max={max} totals={totals:?}"
        );
    }
}
