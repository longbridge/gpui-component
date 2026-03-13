use std::sync::Arc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, Entity, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, InteractiveElement, IntoElement, LayoutId, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, SharedString, StyleRefinement, Styled, Window, div,
};

use crate::StyledExt;
use crate::scroll::ScrollableElement;
use crate::text::TextViewFormat;
use crate::text::node::CodeBlock;
use crate::text::state::TextViewState;
use crate::{global_state::GlobalState, text::TextViewStyle};

/// Type for code block actions generator function.
pub(crate) type CodeBlockActionsFn =
    dyn Fn(&CodeBlock, &mut Window, &mut App) -> AnyElement + Send + Sync;

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
    scrollable: bool,
    code_block_actions: Option<Arc<CodeBlockActionsFn>>,
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
            scrollable: false,
            code_block_actions: None,
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
            scrollable: false,
            code_block_actions: None,
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
            scrollable: false,
            code_block_actions: None,
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

impl Element for TextView {
    type RequestLayoutState = TextViewLayoutState;
    type PrepaintState = Hitbox;

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

        state.update(cx, |state, cx| {
            state.code_block_actions = self.code_block_actions.clone();
            state.selectable = self.selectable;
            state.scrollable = self.scrollable;
            state.text_view_style = self.text_view_style.clone();

            if let Some(text) = self.text.clone() {
                state.set_text(text.as_str(), cx);
            }
        });

        let focus_handle = state.read(cx).focus_handle.clone();
        let list_state = state.read(cx).list_state.clone();

        let mut el = div()
            .key_context("TextView")
            .track_focus(&focus_handle)
            .when(self.scrollable, |this| {
                this.size_full().vertical_scrollbar(&list_state)
            })
            .relative()
            .on_action(window.listener_for(&state, TextViewState::on_action_copy))
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
        request_layout.element.prepaint(window, cx);
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = &request_layout.state;
        GlobalState::global_mut(cx)
            .text_view_state_stack
            .push(state.clone());
        request_layout.element.paint(window, cx);
        GlobalState::global_mut(cx).text_view_state_stack.pop();

        if self.selectable {
            let is_selecting = state.read(cx).is_selecting;
            let has_selection = state.read(cx).has_selection();
            let parent_view_id = window.current_view();

            window.on_mouse_event({
                let state = state.clone();
                let hitbox = hitbox.clone();
                move |event: &MouseDownEvent, phase, window, cx| {
                    if !phase.bubble() || !hitbox.is_hovered(window) {
                        return;
                    }

                    state.update(cx, |state, _| {
                        state.start_selection(event.position);
                    });
                    cx.notify(parent_view_id);
                }
            });

            if is_selecting {
                // move to update end position.
                window.on_mouse_event({
                    let state = state.clone();
                    move |event: &MouseMoveEvent, phase, _, cx| {
                        if !phase.bubble() {
                            return;
                        }

                        state.update(cx, |state, _| {
                            state.update_selection(event.position);
                        });
                        cx.notify(parent_view_id);
                    }
                });

                // up to end selection
                window.on_mouse_event({
                    let state = state.clone();
                    move |_: &MouseUpEvent, phase, _, cx| {
                        if !phase.bubble() {
                            return;
                        }

                        state.update(cx, |state, _| {
                            state.end_selection();
                        });
                        cx.notify(parent_view_id);
                    }
                });
            }

            if has_selection {
                // down outside to clear selection
                window.on_mouse_event({
                    let state = state.clone();
                    let hitbox = hitbox.clone();
                    move |_: &MouseDownEvent, _, window, cx| {
                        if hitbox.is_hovered(window) {
                            return;
                        }

                        state.update(cx, |state, _| {
                            state.clear_selection();
                        });
                        cx.notify(parent_view_id);
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TextView;
    use crate::text::TextViewState;
    use gpui::{
        AppContext as _, Context, Entity, IntoElement, Modifiers, MouseButton, ParentElement as _,
        Render, Styled as _, TestAppContext, VisualTestContext, Window, div, point, px,
    };

    struct TextViewTestRoot {
        text_view: Entity<TextViewState>,
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
}
