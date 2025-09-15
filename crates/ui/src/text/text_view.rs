use std::{rc::Rc, sync::Arc, time::Instant};

use gpui::{
    div, px, rems, AnyElement, App, Bounds, ClipboardItem, Element, ElementId, Entity, FocusHandle,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, KeyBinding, LayoutId,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Rems, RenderOnce,
    SharedString, Size, Window,
};

use super::format::{html::HtmlElement, markdown::MarkdownElement};
use crate::{
    global_state::GlobalState,
    highlighter::HighlightTheme,
    input::{self},
    text::node::{self, NodeContext},
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

#[derive(Default, Clone, PartialEq)]
pub(crate) struct TextViewState {
    root: Option<Result<Rc<node::Node>, SharedString>>,
    pub(crate) node_cx: Rc<node::NodeContext>,

    raw: SharedString,
    focus_handle: Option<FocusHandle>,

    /// The bounds of the text view
    bounds: Bounds<Pixels>,
    /// The local (in TextView) position of the selection.
    selection_positions: (Option<Point<Pixels>>, Option<Point<Pixels>>),
    /// Is current in selection.
    is_selecting: bool,
    is_selectable: bool,

    _last_parsed: Option<Instant>,
}

impl TextViewState {
    pub fn new(cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            raw: SharedString::default(),
            focus_handle: Some(focus_handle),
            root: None,
            node_cx: Rc::new(NodeContext::default()),
            _last_parsed: None,
            bounds: Bounds::default(),
            selection_positions: (None, None),
            is_selecting: false,
            is_selectable: false,
        }
    }
}

impl TextViewState {
    pub(super) fn root(&self) -> Result<Rc<node::Node>, SharedString> {
        self.root
            .clone()
            .expect("The `root` should call `parse_if_needed` before to use.")
    }

    pub(super) fn parse_if_needed(
        &mut self,
        new_text: SharedString,
        is_html: bool,
        style: &TextViewStyle,
        cx: &mut App,
    ) {
        let is_changed = self.raw != new_text || self.node_cx.style != *style;

        if self.root.is_some() && !is_changed {
            return;
        }

        if let Some(last_parsed) = self._last_parsed {
            if last_parsed.elapsed().as_millis() < 500 {
                return;
            }
        }

        let mut node_cx = NodeContext::default();
        node_cx.style = style.clone();
        self.raw = new_text;
        // NOTE: About 100ms
        // let measure = crate::Measure::new("parse_markdown");
        self.root = Some(
            if is_html {
                super::format::html::parse(&self.raw, &mut node_cx)
            } else {
                super::format::markdown::parse(&self.raw, &style, &mut node_cx, cx)
            }
            .map(Rc::new),
        );
        self.node_cx = Rc::new(node_cx);
        // measure.end();
        self._last_parsed = Some(Instant::now());
        self.clear_selection();
    }

    /// Save bounds and unselect if bounds changed.
    fn update_bounds(&mut self, bounds: Bounds<Pixels>) {
        if self.bounds.size != bounds.size {
            self.clear_selection();
        }
        self.bounds = bounds;
    }

    fn clear_selection(&mut self) {
        self.selection_positions = (None, None);
        self.is_selecting = false;
    }

    fn start_selection(&mut self, pos: Point<Pixels>) {
        let pos = pos - self.bounds.origin;
        self.selection_positions = (Some(pos), Some(pos));
        self.is_selecting = true;
    }

    fn update_selection(&mut self, pos: Point<Pixels>) {
        let pos = pos - self.bounds.origin;
        if let (Some(start), Some(_)) = self.selection_positions {
            self.selection_positions = (Some(start), Some(pos))
        }
    }

    fn end_selection(&mut self) {
        self.is_selecting = false;
    }

    pub(crate) fn has_selection(&self) -> bool {
        if let (Some(start), Some(end)) = self.selection_positions {
            start != end
        } else {
            false
        }
    }

    pub(crate) fn is_selectable(&self) -> bool {
        self.is_selectable
    }

    /// Return the bounds of the selection in window coordinates.
    pub(crate) fn selection_bounds(&self) -> Bounds<Pixels> {
        if let (Some(start), Some(end)) = self.selection_positions {
            let start = start + self.bounds.origin;
            let end = end + self.bounds.origin;

            let origin = Point {
                x: start.x.min(end.x),
                y: start.y.min(end.y),
            };
            let size = Size {
                width: (start.x - end.x).abs(),
                height: (start.y - end.y).abs(),
            };

            return Bounds { origin, size };
        }

        Bounds::default()
    }

    fn selection_text(&self) -> Option<String> {
        let Some(Ok(root)) = &self.root else {
            return None;
        };

        Some(root.selected_text())
    }
}

#[derive(IntoElement, Clone)]
pub enum Text {
    String(SharedString),
    TextView(Box<TextView>),
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
        Self::TextView(Box::new(e))
    }
}

impl Text {
    /// Set the style for [`TextView`].
    ///
    /// Do nothing if this is `String`.
    pub fn style(self, style: TextViewStyle) -> Self {
        match self {
            Self::String(s) => Self::String(s),
            Self::TextView(e) => Self::TextView(Box::new(e.style(style))),
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
    /// Function to calculate heading font size based on heading level (1-6).
    ///
    /// The first parameter is the heading level (1-6), the second parameter is the base font size.
    /// The second parameter is the base font size.
    pub heading_font_size: Option<Rc<dyn Fn(u8, Pixels) -> Pixels + 'static>>,
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
            heading_font_size: None,
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

    pub fn heading_font_size<F>(mut self, f: F) -> Self
    where
        F: Fn(u8, Pixels) -> Pixels + 'static,
    {
        self.heading_font_size = Some(Rc::new(f));
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
        let state =
            window.use_keyed_state(SharedString::from(format!("{}/state", id)), cx, |_, cx| {
                TextViewState::new(cx)
            });
        Self {
            id,
            state: state.clone(),
            element: TextViewElement::Markdown(MarkdownElement::new(raw, state)),
            selectable: false,
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
        let state =
            window.use_keyed_state(SharedString::from(format!("{}/state", id)), cx, |_, cx| {
                TextViewState::new(cx)
            });

        Self {
            id,
            state: state.clone(),
            element: TextViewElement::Html(HtmlElement::new(raw, state)),
            selectable: false,
        }
    }

    /// Set the text view to be selectable, default is false.
    pub fn selectable(mut self) -> Self {
        self.selectable = true;
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
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let focus_handle = self
            .state
            .read(cx)
            .focus_handle
            .as_ref()
            .expect("focus_handle should init by TextViewState::new");

        let mut el = div()
            .key_context(CONTEXT)
            .track_focus(focus_handle)
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
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        request_layout.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let entity_id = window.current_view();
        let is_selectable = self.selectable;

        self.state.update(cx, |state, _| {
            state.update_bounds(bounds);
            state.is_selectable = is_selectable;
        });

        GlobalState::global_mut(cx)
            .text_view_state_stack
            .push(self.state.clone());
        request_layout.paint(window, cx);
        GlobalState::global_mut(cx).text_view_state_stack.pop();

        if self.selectable {
            let is_selecting = self.state.read(cx).is_selecting;
            let has_selection = self.state.read(cx).has_selection();

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

            if has_selection {
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

#[cfg(test)]
mod tests {
    use gpui::{point, px, size, Bounds};

    use crate::text::TextViewState;

    #[test]
    fn test_text_view_state_selection_bounds() {
        assert_eq!(
            TextViewState {
                selection_positions: (None, None),
                ..Default::default()
            }
            .selection_bounds(),
            Bounds::default()
        );
        assert_eq!(
            TextViewState {
                selection_positions: (None, Some(point(px(10.), px(20.)))),
                ..Default::default()
            }
            .selection_bounds(),
            Bounds::default()
        );
        assert_eq!(
            TextViewState {
                selection_positions: (Some(point(px(10.), px(20.))), None),
                ..Default::default()
            }
            .selection_bounds(),
            Bounds::default()
        );

        // 10,10 start
        //   |------|
        //   |      |
        //   |------|
        //         50,50
        assert_eq!(
            TextViewState {
                selection_positions: (Some(point(px(10.), px(10.))), Some(point(px(50.), px(50.)))),
                ..Default::default()
            }
            .selection_bounds(),
            Bounds {
                origin: point(px(10.), px(10.)),
                size: size(px(40.), px(40.))
            }
        );
        // 10,10
        //   |------|
        //   |      |
        //   |------|
        //         50,50 start
        assert_eq!(
            TextViewState {
                selection_positions: (Some(point(px(50.), px(50.))), Some(point(px(10.), px(10.))),),
                ..Default::default()
            }
            .selection_bounds(),
            Bounds {
                origin: point(px(10.), px(10.)),
                size: size(px(40.), px(40.))
            }
        );
        //        50,10 start
        //   |------|
        //   |      |
        //   |------|
        // 10,50
        assert_eq!(
            TextViewState {
                selection_positions: (Some(point(px(50.), px(10.))), Some(point(px(10.), px(50.)))),
                ..Default::default()
            }
            .selection_bounds(),
            Bounds {
                origin: point(px(10.), px(10.)),
                size: size(px(40.), px(40.))
            }
        );
        //        50,10
        //   |------|
        //   |      |
        //   |------|
        // 10,50 start
        assert_eq!(
            TextViewState {
                selection_positions: (Some(point(px(10.), px(50.))), Some(point(px(50.), px(10.)))),
                ..Default::default()
            }
            .selection_bounds(),
            Bounds {
                origin: point(px(10.), px(10.)),
                size: size(px(40.), px(40.))
            }
        );
    }
}
