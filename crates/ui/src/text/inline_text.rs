use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    point, px, AnyElement, AvailableSpace, Element, ElementId, HighlightStyle, InteractiveText,
    IntoElement, MouseDownEvent, Pixels, Point, SharedString, Size, StyledText, TextLayout,
    WhiteSpace, WrappedLine,
};
use smallvec::SmallVec;

use crate::{input::Selection, text::element::LinkMark};

pub(super) struct InlineText {
    id: ElementId,
    text: SharedString,
    links: Vec<(Range<usize>, LinkMark)>,
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
            links,
            highlights,
            styled_text: StyledText::new(text),
        }
    }
}

#[derive(Default, Clone)]
pub struct InlineTextState {
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
    type PrepaintState = ();

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
            .prepaint(id, inspector_id, bounds, request_layout, window, cx)
    }

    fn paint(
        &mut self,
        global_id: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) {
        let line_height = window.line_height();
        let origin = bounds.origin;
        let last_layout = self.styled_text.layout();

        self.styled_text.paint(
            global_id,
            None,
            bounds,
            request_layout,
            prepaint,
            window,
            cx,
        );

        window.with_element_state(global_id.unwrap(), move |state, window| {
            let state: InlineTextState = state.unwrap_or_default();

            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if !bounds.contains(&event.position) || !phase.bubble() {
                    return;
                }

                let pos = event.position - bounds.origin;
            });

            ((), state)
        });
    }
}
