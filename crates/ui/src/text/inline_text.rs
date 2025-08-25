use std::{
    cell::{Cell, RefCell},
    ops::Range,
    rc::Rc,
};

use gpui::{
    point, px, quad, BorderStyle, Bounds, CursorStyle, Edges, Element, ElementId, HighlightStyle,
    Hitbox, HitboxBehavior, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder,
    Pixels, Point, SharedString, StyledText, TextLayout, WrappedLine,
};

use crate::{
    input::{Cursor, Selection},
    text::element::LinkMark,
    ActiveTheme,
};

#[derive(Debug, Default, PartialEq, Clone)]
pub struct InlineTextState {
    hovered_index: Rc<Cell<Option<usize>>>,
    is_selection: Rc<Cell<bool>>,
    selection: Rc<Cell<Option<Selection>>>,
}

impl InlineTextState {
    fn start_selection(&self, index: usize) {
        self.is_selection.set(true);
        self.selection.set(Some(Selection {
            start: Cursor::new(index),
            end: Cursor::new(index),
        }));
    }

    fn update_selection(&self, index: usize) {
        if let Some(mut selection) = self.selection.get() {
            selection.end = Cursor::new(index);
            self.selection.set(Some(selection));
        }
    }

    fn end_selection(&self) {
        self.is_selection.set(false);
    }

    pub fn selection(&self) -> Option<Selection> {
        self.selection.get()
    }
}

pub(super) struct InlineText {
    id: ElementId,
    text: SharedString,
    links: Rc<Vec<(Range<usize>, LinkMark)>>,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
    state: InlineTextState,
    styled_text: StyledText,
}

impl InlineText {
    pub(super) fn new(
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        links: Vec<(Range<usize>, LinkMark)>,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
        state: InlineTextState,
    ) -> Self {
        let text: SharedString = text.into();
        Self {
            id: id.into(),
            text: text.clone(),
            links: Rc::new(links),
            highlights,
            styled_text: StyledText::new(text),
            state,
        }
    }

    pub fn link_for_position(
        layout: &TextLayout,
        links: &Vec<(Range<usize>, LinkMark)>,
        position: Point<Pixels>,
    ) -> Option<LinkMark> {
        if let Ok(offset) = layout.index_for_position(position) {
            for (range, link) in links.iter() {
                if range.contains(&offset) {
                    return Some(link.clone());
                }
            }
        }
        None
    }
}

impl IntoElement for InlineText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for InlineText {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_element_id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let text_style = window.text_style();

        let mut runs = Vec::new();
        let mut ix = 0;
        for (range, highlight) in self.highlights.iter() {
            if ix < range.start {
                runs.push(text_style.clone().to_run(range.start - ix));
            }
            runs.push(
                text_style
                    .clone()
                    .highlight(highlight.clone())
                    .to_run(range.len()),
            );
            ix = range.end;
        }
        if ix < self.text.len() {
            runs.push(text_style.to_run(self.text.len() - ix));
        }

        self.styled_text = StyledText::new(self.text.clone()).with_runs(runs);
        let (layout_id, _) =
            self.styled_text
                .request_layout(global_element_id, inspector_id, window, cx);

        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> Self::PrepaintState {
        self.styled_text
            .prepaint(id, inspector_id, bounds, request_layout, window, cx);

        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        hitbox
    }

    fn paint(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) {
        let current_view = window.current_view();
        let hitbox = prepaint.clone();
        self.styled_text
            .paint(global_id, None, bounds, request_layout, &mut (), window, cx);
        let text_layout = self.styled_text.layout().clone();
        let is_selection = self.state.is_selection.get();

        // link cursor pointer
        let mouse_position = window.mouse_position();
        if let Some(_) = Self::link_for_position(&text_layout, &self.links, mouse_position) {
            window.set_cursor_style(CursorStyle::PointingHand, &hitbox);
        } else {
            if is_selection {
                window.set_cursor_style(CursorStyle::IBeam, &hitbox);
            }
        }

        if let Some(selection) = self.state.selection() {
            let start_position = text_layout.position_for_index(selection.start.offset());
            let end_position = text_layout.position_for_index(selection.end.offset());
            let line_height = text_layout.line_height();
            if let Some(start_position) = start_position {
                if let Some(end_position) = end_position {
                    if start_position.y == end_position.y {
                        window.paint_quad(quad(
                            Bounds::from_corners(
                                start_position,
                                point(end_position.x, end_position.y + line_height),
                            ),
                            px(0.),
                            cx.theme().selection,
                            Edges::default(),
                            gpui::transparent_black(),
                            BorderStyle::default(),
                        ));
                    } else {
                        window.paint_quad(quad(
                            Bounds::from_corners(
                                start_position,
                                point(bounds.right(), start_position.y + line_height),
                            ),
                            px(0.),
                            cx.theme().selection,
                            Edges::default(),
                            gpui::transparent_black(),
                            BorderStyle::default(),
                        ));

                        if end_position.y > start_position.y + line_height {
                            window.paint_quad(quad(
                                Bounds::from_corners(
                                    point(bounds.left(), start_position.y + line_height),
                                    point(bounds.right(), end_position.y),
                                ),
                                px(0.),
                                cx.theme().selection,
                                Edges::default(),
                                gpui::transparent_black(),
                                BorderStyle::default(),
                            ));
                        }

                        window.paint_quad(quad(
                            Bounds::from_corners(
                                point(bounds.left(), end_position.y),
                                point(end_position.x, end_position.y + line_height),
                            ),
                            px(0.),
                            cx.theme().selection,
                            Edges::default(),
                            gpui::transparent_black(),
                            BorderStyle::default(),
                        ));
                    }
                }
            }
        }

        // mouse move to notify update when hovering over different links
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let text_layout = text_layout.clone();
            let state = self.state.clone();
            let hovered_index = state.hovered_index.clone();

            move |event: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() || !hitbox.is_hovered(window) {
                    return;
                }

                let current = hovered_index.get();
                let updated = text_layout.index_for_position(event.position).ok();
                if current != updated {
                    hovered_index.set(updated);
                    cx.notify(current_view);
                }

                // update selection if in selection mode
                if state.is_selection.get() {
                    if let Ok(index) = text_layout.index_for_position(event.position) {
                        state.update_selection(index);
                        cx.notify(current_view);
                    }
                }
            }
        });

        // mouse down to start selection
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let text_layout = text_layout.clone();
            let state = self.state.clone();

            move |event: &MouseDownEvent, phase, window, cx| {
                if !hitbox.is_hovered(window) || !phase.bubble() {
                    return;
                }

                if let Ok(index) = text_layout.index_for_position(event.position) {
                    state.start_selection(index);
                    cx.notify(current_view);
                }
            }
        });

        // click
        window.on_mouse_event({
            let links = self.links.clone();
            let text_layout = text_layout.clone();
            let state = self.state.clone();

            move |event: &MouseUpEvent, phase, _, cx| {
                if state.is_selection.get() {
                    state.end_selection();
                    cx.notify(current_view);
                }

                if !bounds.contains(&event.position) || !phase.bubble() {
                    return;
                }

                if let Some(link) = Self::link_for_position(&text_layout, &links, event.position) {
                    cx.stop_propagation();
                    cx.open_url(&link.url);
                }
            }
        });
    }
}
