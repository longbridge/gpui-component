use std::{
    cell::{Cell, RefCell},
    ops::Range,
    rc::Rc,
};

use gpui::{
    point, px, AnyElement, AvailableSpace, Bounds, ContentMask, CursorStyle, Element, ElementId,
    HighlightStyle, Hitbox, HitboxBehavior, InteractiveText, IntoElement, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString, Size, StyledText, TextLayout,
    WhiteSpace, WrappedLine,
};
use smallvec::SmallVec;

use crate::{input::Selection, text::element::LinkMark};

pub(super) struct InlineText {
    id: ElementId,
    text: SharedString,
    links: Rc<Vec<(Range<usize>, LinkMark)>>,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
    styled_text: StyledText,
}

impl InlineText {
    pub(super) fn new(
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        links: Vec<(Range<usize>, LinkMark)>,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
    ) -> Self {
        let text: SharedString = text.into();
        Self {
            id: id.into(),
            text: text.clone(),
            links: Rc::new(links),
            highlights,
            styled_text: StyledText::new(text),
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

#[derive(Default, Clone)]
pub struct InlineTextState {
    hovered_index: Rc<Cell<Option<usize>>>,
    selection: Rc<RefCell<Selection>>,
}

pub struct LastLayout {
    lines: Rc<SmallVec<[WrappedLine; 1]>>,
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

        // link cursor pointer
        let mouse_position = window.mouse_position();
        if let Some(_) = Self::link_for_position(&text_layout, &self.links, mouse_position) {
            window.set_cursor_style(CursorStyle::PointingHand, &hitbox);
        }

        window.with_element_state(global_id.unwrap(), move |state, window| {
            let state: InlineTextState = state.unwrap_or_default();

            // mouse move to notify update when hovering over different links
            window.on_mouse_event({
                let hitbox = hitbox.clone();
                let text_layout = text_layout.clone();
                let hovered_index = state.hovered_index.clone();
                move |event: &MouseMoveEvent, phase, window, cx| {
                    if phase.bubble() && hitbox.is_hovered(window) {
                        let current = hovered_index.get();
                        let updated = text_layout.index_for_position(event.position).ok();
                        if current != updated {
                            hovered_index.set(updated);
                            cx.notify(current_view);
                        }
                    }
                }
            });

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
                        cx.open_url(&link.url);
                    }
                }
            });

            ((), state)
        });
    }
}
