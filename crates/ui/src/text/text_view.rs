use std::sync::Arc;

use gpui::{
    canvas, div, prelude::FluentBuilder as _, px, rems, AnyElement, App, Bounds, Element,
    ElementId, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Pixels,
    Point, Rems, RenderOnce, SharedString, Styled, Window,
};

use crate::{global_state::GlobalState, highlighter::HighlightTheme, Root};

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
#[derive(IntoElement, Clone)]
pub struct TextView {
    id: ElementId,
    element: TextViewElement,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TextViewState {
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

impl RenderOnce for TextView {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let view_id = window.current_view();
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| TextViewState::default());
        let is_selection = state.read(cx).is_selection;

        GlobalState::global_mut(cx).with_text_view_state(state.clone(), |_| {
            tracing::info!("-------------------- TextView element.");
            div()
                .id(self.id)
                .relative()
                .child(self.element)
                .on_mouse_down(MouseButton::Left, {
                    let state = state.clone();
                    let view_id = view_id.clone();
                    move |event, _, cx| {
                        state.update(cx, |state, _| {
                            state.start_selection(event.position);
                        });
                        cx.notify(view_id);
                    }
                })
                .when(is_selection, |this| {
                    this.on_mouse_move({
                        let state = state.clone();
                        let view_id = view_id.clone();
                        move |event, _, cx| {
                            if state.read(cx).is_selection {
                                state.update(cx, |state, _| {
                                    state.update_selection(event.position);
                                });
                                cx.notify(view_id);
                            }
                        }
                    })
                    .on_mouse_up(MouseButton::Left, {
                        let state = state.clone();
                        let view_id = view_id.clone();
                        move |_, _, cx| {
                            state.update(cx, |state, _| {
                                state.end_selection();
                            });
                            cx.notify(view_id);
                        }
                    })
                    .on_mouse_down_out({
                        let state = state.clone();
                        let view_id = view_id.clone();
                        move |_, _, cx| {
                            state.update(cx, |state, _| {
                                state.clear_selection();
                            });
                            cx.notify(view_id);
                        }
                    })
                })
                .child(
                    canvas(
                        {
                            let state = state.clone();
                            move |bounds, _, cx| state.update(cx, |r, _| r.bounds = bounds)
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
        })
    }
}
