use std::sync::Arc;

use gpui::{
    px, rems, AnyElement, App, Bounds, Element, ElementId, IntoElement, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, Rems, RenderOnce, SharedString, Window,
};

use crate::{global_state::GlobalState, highlighter::HighlightTheme};

use super::{html::HtmlElement, markdown::MarkdownElement};

#[derive(IntoElement, Clone)]
enum TextViewElement {
    Markdown(MarkdownElement),
    Html(HtmlElement),
}

impl RenderOnce for TextViewElement {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        match self {
            Self::Markdown(el) => el.render(window, cx).into_any_element(),
            Self::Html(el) => el.render(window, cx).into_any_element(),
        }
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
    element: TextViewElement,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextViewState {
    /// The bounds of the text view
    pub(crate) bounds: Bounds<Pixels>,
    /// The local (in TextView) position of the selection.
    pub(crate) selection_pos: (Option<Point<Pixels>>, Option<Point<Pixels>>),
    /// Is current in selection mode.
    pub(crate) is_selection: bool,
}

impl TextViewState {
    pub(crate) fn clear_selection(&mut self) {
        self.selection_pos = (None, None);
        self.is_selection = false;
    }

    pub(crate) fn start_selection(&mut self, pos: Point<Pixels>) {
        let pos = pos - self.bounds.origin;
        self.selection_pos = (Some(pos), Some(pos));
        self.is_selection = true;
    }

    pub(crate) fn update_selection(&mut self, pos: Point<Pixels>) {
        let pos = pos - self.bounds.origin;
        if let (Some(start), Some(_)) = self.selection_pos {
            self.selection_pos = (Some(start), Some(pos))
        }
    }

    pub(crate) fn end_selection(&mut self) {
        self.is_selection = false;
    }
}

#[derive(IntoElement, Clone)]
pub enum Text {
    String(SharedString),
    TextView(TextView),
}

impl From<SharedString> for Text {
    fn from(s: SharedString) -> Self {
        Self::String(s)
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Self::String(SharedString::from(s.to_string()))
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Self::String(s.into())
    }
}

impl From<TextView> for Text {
    fn from(e: TextView) -> Self {
        Self::TextView(e)
    }
}

impl Text {
    /// Set the style for [`TextView`].
    ///
    /// Do nothing if this is `String`.
    pub fn style(self, style: TextViewStyle) -> Self {
        match self {
            Self::String(s) => Self::String(s),
            Self::TextView(e) => Self::TextView(e.style(style)),
        }
    }
}

impl RenderOnce for Text {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self {
            Self::String(s) => s.into_any_element(),
            Self::TextView(e) => e.into_any_element(),
        }
    }
}

/// TextViewStyle used to customize the style for [`TextView`].
#[derive(Clone)]
pub struct TextViewStyle {
    /// Gap of each paragraphs, default is 1 rem.
    pub paragraph_gap: Rems,
    /// Base font size for headings, default is 14px.
    pub heading_base_font_size: Pixels,
    /// Highlight theme for code blocks. Default: [`HighlightTheme::default_light()`]
    pub highlight_theme: Arc<HighlightTheme>,
    pub is_dark: bool,
}

impl PartialEq for TextViewStyle {
    fn eq(&self, other: &Self) -> bool {
        self.paragraph_gap == other.paragraph_gap
            && self.heading_base_font_size == other.heading_base_font_size
            && self.highlight_theme == other.highlight_theme
    }
}

impl Default for TextViewStyle {
    fn default() -> Self {
        Self {
            paragraph_gap: rems(1.),
            heading_base_font_size: px(14.),
            highlight_theme: HighlightTheme::default_light().clone(),
            is_dark: false,
        }
    }
}

impl TextViewStyle {
    /// Set paragraph gap, default is 1 rem.
    pub fn paragraph_gap(mut self, gap: Rems) -> Self {
        self.paragraph_gap = gap;
        self
    }
}

impl TextView {
    /// Create a new markdown text view.
    pub fn markdown(id: impl Into<ElementId>, raw: impl Into<SharedString>) -> Self {
        let id: ElementId = id.into();
        let el_id = SharedString::from(format!("{}/markdown", id));

        Self {
            id,
            element: TextViewElement::Markdown(MarkdownElement::new(el_id, raw)),
        }
    }

    /// Create a new html text view.
    pub fn html(id: impl Into<ElementId>, raw: impl Into<SharedString>) -> Self {
        let id: ElementId = id.into();
        let el_id = SharedString::from(format!("{}/html", id));

        Self {
            id,
            element: TextViewElement::Html(HtmlElement::new(el_id, raw)),
        }
    }

    /// Set the source text of the text view.
    pub fn text(mut self, raw: impl Into<SharedString>) -> Self {
        self.element = match self.element {
            TextViewElement::Markdown(el) => TextViewElement::Markdown(el.text(raw)),
            TextViewElement::Html(el) => TextViewElement::Html(el.text(raw)),
        };
        self
    }

    /// Set [`TextViewStyle`].
    pub fn style(mut self, style: TextViewStyle) -> Self {
        self.element = match self.element {
            TextViewElement::Markdown(el) => TextViewElement::Markdown(el.style(style)),
            TextViewElement::Html(el) => TextViewElement::Html(el.style(style)),
        };
        self
    }
}

impl IntoElement for TextView {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextView {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut el = self.element.clone().into_any_element();
        let layout_id = el.request_layout(window, cx);
        (layout_id, el)
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        request_layout.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let entity_id = window.current_view();
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| TextViewState::default());
        let is_selection = state.read(cx).is_selection;

        state.update(cx, |state, _| state.bounds = bounds);

        GlobalState::global_mut(cx)
            .text_view_state_stack
            .push(state.clone());
        request_layout.paint(window, cx);
        GlobalState::global_mut(cx).text_view_state_stack.pop();

        window.on_mouse_event({
            let state = state.clone();
            move |event: &MouseDownEvent, phase, _, cx| {
                if !bounds.contains(&event.position) || !phase.bubble() {
                    return;
                }

                state.update(cx, |state, _| {
                    state.start_selection(event.position);
                });
                cx.notify(entity_id);
            }
        });

        if is_selection {
            // move to update end postion.
            window.on_mouse_event({
                let state = state.clone();
                move |event: &MouseMoveEvent, phase, _, cx| {
                    if !bounds.contains(&event.position) || !phase.bubble() {
                        return;
                    }

                    state.update(cx, |state, _| {
                        state.update_selection(event.position);
                    });
                    cx.notify(entity_id);
                }
            });

            // up to end selection
            window.on_mouse_event({
                let state = state.clone();
                move |event: &MouseUpEvent, phase, _, cx| {
                    if !bounds.contains(&event.position) || !phase.bubble() {
                        return;
                    }

                    state.update(cx, |state, _| {
                        state.end_selection();
                    });
                    cx.notify(entity_id);
                }
            });

            // down outside to clear selection
            window.on_mouse_event({
                let state = state.clone();
                move |event: &MouseDownEvent, phase, _, cx| {
                    if bounds.contains(&event.position) || !phase.bubble() {
                        return;
                    }

                    state.update(cx, |state, _| {
                        state.clear_selection();
                    });
                    cx.notify(entity_id);
                }
            });
        }
    }
}
