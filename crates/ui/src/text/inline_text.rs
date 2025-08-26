use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    point, px, quad, App, BorderStyle, Bounds, CursorStyle, Edges, Element, ElementId,
    GlobalElementId, HighlightStyle, Hitbox, HitboxBehavior, InspectorElementId, IntoElement,
    LayoutId, MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString, StyledText, TextLayout,
    Window,
};

use crate::{
    global_state::GlobalState,
    input::{Cursor, Selection},
    text::element::LinkMark,
    ActiveTheme,
};

#[derive(Debug, Default, PartialEq, Clone)]
pub struct InlineTextState {
    hovered_index: Rc<RefCell<Option<usize>>>,
    pub(super) text: Rc<RefCell<SharedString>>,
    pub(super) selection: Rc<RefCell<Option<Selection>>>,
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
        text: SharedString,
        links: Vec<(Range<usize>, LinkMark)>,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
        state: InlineTextState,
    ) -> Self {
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
        global_element_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
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
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.styled_text
            .prepaint(id, inspector_id, bounds, &mut (), window, cx);

        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        hitbox
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let current_view = window.current_view();
        let hitbox = prepaint;
        let line_height = window.line_height();

        let state = self.state.clone();

        self.styled_text
            .paint(global_id, None, bounds, &mut (), &mut (), window, cx);
        let text_layout = self.styled_text.layout().clone();

        // layout selections
        let mut is_selection = false;
        let mut selection = None;
        if let Some(text_view_state) = GlobalState::global(cx).text_view_state() {
            let text_view_state = text_view_state.read(cx);
            if text_view_state.has_selection() {
                is_selection = true;
                let mut selection_bounds = Bounds::default();
                let selection_position = text_view_state.selection_position();
                if let Some(start_pos) = &selection_position.0 {
                    if let Some(end_pos) = &selection_position.1 {
                        selection_bounds = Bounds::from_corners(*start_pos, *end_pos);
                    }
                }

                // Use for debug selection bounds
                // window.paint_quad(gpui::PaintQuad {
                //     bounds: selection_bounds,
                //     background: cx.theme().blue.alpha(0.01).into(),
                //     corner_radii: gpui::Corners::default(),
                //     border_color: gpui::transparent_black(),
                //     border_style: BorderStyle::default(),
                //     border_widths: gpui::Edges::all(px(0.)),
                // });

                fn point_in_column_selection(
                    pos: Point<Pixels>,
                    selection_bounds: &Bounds<Pixels>,
                    line_height: Pixels,
                ) -> bool {
                    let top = selection_bounds.top();
                    let bottom = selection_bounds.bottom();
                    let left = selection_bounds.left();
                    let right = selection_bounds.right();

                    // Out of the vertical bounds
                    if pos.x < top || pos.y >= bottom {
                        return false;
                    }

                    let single_line = (bottom - top) <= line_height;

                    if single_line {
                        // If it's a single line selection, just check horizontal bounds
                        return pos.x >= left && pos.x <= right;
                    }

                    let is_first_line = pos.y + line_height >= top && pos.y < top + line_height;
                    let is_last_line = pos.y >= bottom - line_height && pos.y < bottom;

                    if is_first_line {
                        // First line: from left to the end of the line
                        return pos.x >= left;
                    } else if is_last_line {
                        // Last line: from the start of the line to right
                        return pos.x <= right;
                    } else {
                        // Other lines in between: full line selection
                        return pos.y >= top;
                    }
                }

                let mut offset = 0;
                let mut chars = self.text.chars().peekable();
                while let Some(c) = chars.next() {
                    let Some(pos) = text_layout.position_for_index(offset) else {
                        offset += c.len_utf8();
                        continue;
                    };

                    if point_in_column_selection(pos, &selection_bounds, line_height) {
                        if selection.is_none() {
                            selection = Some((offset, offset));
                        }

                        let next_offset = offset + c.len_utf8();
                        selection.as_mut().unwrap().1 = next_offset;
                    }

                    offset += c.len_utf8();
                }
            }
        }

        *state.selection.borrow_mut() = if let Some(selection) = selection {
            Some(Selection {
                start: Cursor::new(selection.0),
                end: Cursor::new(selection.1),
            })
        } else {
            None
        };

        if is_selection {
            window.set_cursor_style(CursorStyle::IBeam, &hitbox);
        } else {
            // link cursor pointer
            let mouse_position = window.mouse_position();
            if let Some(_) = Self::link_for_position(&text_layout, &self.links, mouse_position) {
                window.set_cursor_style(CursorStyle::PointingHand, &hitbox);
            }
        }

        if let Some(selection) = *state.selection.borrow() {
            let mut start_offset = selection.start.offset();
            let mut end_offset = selection.end.offset();
            if end_offset < start_offset {
                std::mem::swap(&mut start_offset, &mut end_offset);
            }
            let start_position = text_layout.position_for_index(start_offset);
            let end_position = text_layout.position_for_index(end_offset);

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

        // mouse move
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let text_layout = text_layout.clone();
            let hovered_index = state.hovered_index.clone();
            move |event: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() || !hitbox.is_hovered(window) {
                    return;
                }

                let current = *hovered_index.borrow();
                let updated = text_layout.index_for_position(event.position).ok();
                //  notify update when hovering over different links
                if current != updated {
                    *hovered_index.borrow_mut() = updated;
                    cx.notify(current_view);
                }
            }
        });

        if !is_selection {
            // click
            window.on_mouse_event({
                let links = self.links.clone();
                let text_layout = text_layout.clone();

                move |event: &MouseUpEvent, phase, _, cx| {
                    if !bounds.contains(&event.position) || !phase.bubble() {
                        return;
                    }

                    if let Some(link) =
                        Self::link_for_position(&text_layout, &links, event.position)
                    {
                        cx.stop_propagation();
                        cx.open_url(&link.url);
                    }
                }
            });
        }
    }
}
