use std::{sync::Arc, time::Instant};

use gpui::{
    div, px, rems, AnyElement, App, Bounds, ClipboardItem, Element, ElementId, Entity, FocusHandle,
    InteractiveElement, IntoElement, KeyBinding, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Point, Rems, RenderOnce, SharedString, Window,
};

use super::{html::HtmlElement, markdown::MarkdownElement};
use crate::{
    global_state::GlobalState,
    highlighter::HighlightTheme,
    input::{self},
    text::element::{self},
};

const CONTEXT: &'static str = "TextView";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys(vec![
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", input::Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", input::Copy, Some(CONTEXT)),
    ]);
}

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
    state: Entity<TextViewState>,
    element: TextViewElement,
    selectable: bool,
}

#[derive(Clone, PartialEq)]
pub struct TextViewState {
    raw: SharedString,
    focus_handle: FocusHandle,
    pub(super) root: Option<Result<element::Node, SharedString>>,
    style: TextViewStyle,
    _last_parsed: Option<Instant>,

    /// The bounds of the text view
    pub(crate) bounds: Bounds<Pixels>,
    /// The local (in TextView) position of the selection.
    selection_pos: (Option<Point<Pixels>>, Option<Point<Pixels>>),
    /// Is current in selection.
    is_selecting: bool,
}

impl TextViewState {
    pub fn new(cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            raw: SharedString::default(),
            focus_handle,
            root: None,
            style: TextViewStyle::default(),
            _last_parsed: None,
            bounds: Bounds::default(),
            selection_pos: (None, None),
            is_selecting: false,
        }
    }
}

impl TextViewState {
    pub(super) fn parse_if_needed(
        &mut self,
        new_text: SharedString,
        is_html: bool,
        style: &TextViewStyle,
        cx: &mut App,
    ) {
        let is_changed = self.raw != new_text || self.style != *style;

        if self.root.is_some() && !is_changed {
            return;
        }

        if let Some(last_parsed) = self._last_parsed {
            if last_parsed.elapsed().as_millis() < 500 {
                return;
            }
        }

        self.raw = new_text;
        // NOTE: About 100ms
        // let measure = crate::Measure::new("parse_markdown");
        self.root = Some(if is_html {
            super::html::parse_html(&self.raw)
        } else {
            super::markdown::parse_markdown(&self.raw, &style, cx)
        });
        // measure.end();
        self._last_parsed = Some(Instant::now());
        self.style = style.clone();
        self.clear_selection();
    }

    pub(crate) fn clear_selection(&mut self) {
        self.selection_pos = (None, None);
        self.is_selecting = false;
    }

    pub(crate) fn start_selection(&mut self, pos: Point<Pixels>) {
        let pos = pos - self.bounds.origin;
        self.selection_pos = (Some(pos), Some(pos));
        self.is_selecting = true;
    }

    pub(crate) fn update_selection(&mut self, pos: Point<Pixels>) {
        let pos = pos - self.bounds.origin;
        if let (Some(start), Some(_)) = self.selection_pos {
            self.selection_pos = (Some(start), Some(pos))
        }
    }

    pub(crate) fn end_selection(&mut self) {
        self.is_selecting = false;
    }

    pub(crate) fn has_selection(&self) -> bool {
        if let (Some(start), Some(end)) = self.selection_pos {
            start != end
        } else {
            false
        }
    }

    /// Return the position of the selection in window coordinates.
    pub(crate) fn selection_position(&self) -> (Option<Point<Pixels>>, Option<Point<Pixels>>) {
        if let (Some(start), Some(end)) = self.selection_pos {
            let start = start + self.bounds.origin;
            let end = end + self.bounds.origin;

            // return in ordered
            if start.x < end.x || start.y <= end.y {
                return (Some(start), Some(end));
            } else {
                return (Some(end), Some(start));
            }
        } else {
            (None, None)
        }
    }

    pub fn selection_text(&self) -> Option<String> {
        let Some(Ok(root)) = &self.root else {
            return None;
        };

        Some(root.selected_text())
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
    pub fn markdown(
        id: impl Into<ElementId>,
        raw: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let id: ElementId = id.into();
        let state = window.use_keyed_state(id.clone(), cx, |_, cx| TextViewState::new(cx));
        Self {
            id,
            state: state.clone(),
            element: TextViewElement::Markdown(MarkdownElement::new(raw, state)),
            selectable: true,
        }
    }

    /// Create a new html text view.
    pub fn html(
        id: impl Into<ElementId>,
        raw: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let id: ElementId = id.into();
        let state = window.use_keyed_state(id.clone(), cx, |_, cx| TextViewState::new(cx));

        Self {
            id,
            state: state.clone(),
            element: TextViewElement::Html(HtmlElement::new(raw, state)),
            selectable: true,
        }
    }

    /// Set the text view to be selectable, default is true.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
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

    fn on_action_copy(state: &Entity<TextViewState>, cx: &mut App) {
        let Some(selected_text) = state.read(cx).selection_text() else {
            return;
        };

        cx.write_to_clipboard(ClipboardItem::new_string(selected_text.trim().to_string()));
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
        let mut el = div()
            .key_context(CONTEXT)
            .track_focus(&self.state.read(cx).focus_handle)
            .on_action({
                let state = self.state.clone();
                move |_: &input::Copy, _, cx| {
                    Self::on_action_copy(&state, cx);
                }
            })
            .child(self.element.clone())
            .into_any_element();
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
        let is_selecting = self.state.read(cx).is_selecting;

        self.state.update(cx, |state, _| state.bounds = bounds);

        GlobalState::global_mut(cx)
            .text_view_state_stack
            .push(self.state.clone());
        request_layout.paint(window, cx);
        GlobalState::global_mut(cx).text_view_state_stack.pop();

        if self.selectable {
            window.on_mouse_event({
                let state = self.state.clone();
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

            if is_selecting {
                // move to update end position.
                window.on_mouse_event({
                    let state = self.state.clone();
                    move |event: &MouseMoveEvent, _, _, cx| {
                        state.update(cx, |state, _| {
                            state.update_selection(event.position);
                        });
                        cx.notify(entity_id);
                    }
                });

                // up to end selection
                if self.state.read(cx).has_selection() {
                    window.on_mouse_event({
                        let state = self.state.clone();
                        move |_: &MouseUpEvent, _, _, cx| {
                            state.update(cx, |state, _| {
                                state.end_selection();
                            });
                            cx.notify(entity_id);
                        }
                    });
                }
            }

            if self.state.read(cx).has_selection() {
                // down outside to clear selection
                window.on_mouse_event({
                    let state = self.state.clone();
                    move |event: &MouseDownEvent, _, _, cx| {
                        if bounds.contains(&event.position) {
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
}
