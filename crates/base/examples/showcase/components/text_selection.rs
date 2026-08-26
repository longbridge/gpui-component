use std::ops::Range;

use gpui::{
    App, BorderStyle, Bounds, Context, Corners, Edges, Element, ElementId, GlobalElementId, Hitbox,
    InspectorElementId, IntoElement, LayoutId, PaintQuad, ParentElement as _, Pixels, Point,
    SharedString, Styled as _, StyledText, Window, transparent_black,
};
#[cfg(test)]
use gpui_base::ElementExt as _;
use gpui_base::{TextSelection, TextSelectionHandle, TextSelectionRegistration, TextSelectionRun};

use super::*;

const PRODUCT_PARAGRAPH: &str = "Selection should feel like a natural part of reading a product brief. Start in this paragraph, continue into the next renderer, and GPUI preserves the document order while every frame supplies fresh geometry for the same stable selection handle.";
const IMPLEMENTATION_PARAGRAPH: &str = "This second paragraph is deliberately long enough to wrap in the showcase. Drag across the boundary to see one continuous highlight, then use the platform copy shortcut to confirm that the copied result follows the visible reading order rather than renderer ownership.";
const INTERNATIONAL_PARAGRAPH: &str = "International text should remain predictable when a line mixes café, déjà vu, Kraków, naïve, and résumé. Resize the window or drag across several wrapped lines; UTF-8 byte ranges still map back to the correct glyphs without splitting a character.";

struct PlainSelectableText {
    selection: TextSelectionHandle,
    text: SharedString,
    styled_text: StyledText,
    document_order: u64,
}

fn selection_quad_bounds(
    start: Point<Pixels>,
    end: Point<Pixels>,
    bounds: Bounds<Pixels>,
    line_height: Pixels,
) -> Vec<Bounds<Pixels>> {
    if start.y == end.y {
        return vec![Bounds::from_corners(
            start,
            Point::new(end.x, end.y + line_height),
        )];
    }

    let mut quads = vec![Bounds::from_corners(
        start,
        Point::new(bounds.right(), start.y + line_height),
    )];
    if end.y > start.y + line_height {
        quads.push(Bounds::from_corners(
            Point::new(bounds.left(), start.y + line_height),
            Point::new(bounds.right(), end.y),
        ));
    }
    quads.push(Bounds::from_corners(
        Point::new(bounds.left(), end.y),
        Point::new(end.x, end.y + line_height),
    ));
    quads
}

impl PlainSelectableText {
    fn new(
        selection: TextSelectionHandle,
        document_order: u64,
        text: impl Into<SharedString>,
    ) -> Self {
        let text = text.into();
        Self {
            selection,
            styled_text: StyledText::new(text.clone()),
            text,
            document_order,
        }
    }

    fn paint_selection(layout: &gpui::TextLayout, range: Range<usize>, window: &mut Window) {
        let (Some(start), Some(end)) = (
            layout.position_for_index(range.start),
            layout.position_for_index(range.end),
        ) else {
            return;
        };
        let color = gpui::hsla(0.58, 0.85, 0.62, 0.35);
        for bounds in selection_quad_bounds(start, end, layout.bounds(), layout.line_height()) {
            window.paint_quad(PaintQuad {
                bounds,
                background: color.into(),
                corner_radii: Corners::default(),
                border_widths: Edges::default(),
                border_color: transparent_black(),
                border_style: BorderStyle::default(),
            });
        }
    }
}

impl IntoElement for PlainSelectableText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PlainSelectableText {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.styled_text
            .request_layout(id, inspector_id, window, cx)
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
        let hitbox = window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal);
        self.selection.register(
            TextSelectionRegistration::new(hitbox.clone(), bounds)
                .with_document_order(self.document_order)
                .with_text_bounds(vec![bounds]),
            window,
            cx,
        );
        hitbox
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let layout = self.styled_text.layout().clone();
        let selected_text_before = TextSelection::selected_text(window, cx);
        let projection = self.selection.update_runs(
            &[
                TextSelectionRun::new(self.text.clone(), layout.clone(), bounds)
                    .with_document_order(0),
            ],
            cx,
        );
        if selected_text_before != TextSelection::selected_text(window, cx) {
            window.refresh();
        }
        if let Some(range) = projection
            .ranges()
            .iter()
            .next()
            .and_then(|range| range.clone())
        {
            Self::paint_selection(&layout, range, window);
        }
        self.styled_text
            .paint(id, inspector_id, bounds, &mut (), &mut (), window, cx);
    }
}

impl BaseShowcase {
    pub(in super::super) fn text_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.text_selection_active = TextSelection::has_selection(window, cx);
        self.text_selection_text = TextSelection::selected_text(window, cx);

        let active = self.text_selection_active;
        let selected_text = if active {
            self.text_selection_text.clone()
        } else {
            "Drag across any paragraphs to select text.".to_owned()
        };
        let entity = cx.entity().downgrade();

        let footer = div()
            .id("text-selection-footer")
            .h(px(150.))
            .flex_none()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(rgb(0xf5f5f5))
            .border_1()
            .border_color(rgb(0xe5e5e5))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(if active {
                        "Selection active"
                    } else {
                        "No selection"
                    }),
            )
            .child(
                div()
                    .id("text-selection-preview")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .text_color(rgb(0x525252))
                    .child(selected_text),
            )
            .child(
                Button::new("clear-text-selection")
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_center()
                    .self_start()
                    .border_1()
                    .border_color(rgb(0x171717))
                    .child("Clear selection")
                    .on_click(move |_, window, cx| {
                        TextSelection::clear(window, cx);
                        _ = entity.update(cx, |this, cx| {
                            this.text_selection_active = false;
                            this.text_selection_text.clear();
                            cx.notify();
                        });
                    }),
            );
        #[cfg(test)]
        let footer = {
            let bounds = self.text_selection_footer_bounds.clone();
            footer.on_prepaint(move |value, _, _| *bounds.borrow_mut() = Some(value))
        };

        div()
            .id("text-selection-example")
            .w(px(620.))
            .max_w_full()
            .h(px(520.))
            .max_h_full()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .id("text-selection-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.text_selection_scroll)
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(PlainSelectableText::new(
                                self.text_selection_handles[0].clone(),
                                0,
                                "Text selection across renderers",
                            )),
                    )
                    .child(div().text_color(rgb(0x525252)).line_height(px(22.)).child(
                        PlainSelectableText::new(
                            self.text_selection_handles[1].clone(),
                            1,
                            PRODUCT_PARAGRAPH,
                        ),
                    ))
                    .child(div().text_color(rgb(0x525252)).line_height(px(22.)).child(
                        PlainSelectableText::new(
                            self.text_selection_handles[2].clone(),
                            2,
                            IMPLEMENTATION_PARAGRAPH,
                        ),
                    ))
                    .child(div().text_color(rgb(0x525252)).line_height(px(22.)).child(
                        PlainSelectableText::new(
                            self.text_selection_handles[3].clone(),
                            3,
                            INTERNATIONAL_PARAGRAPH,
                        ),
                    )),
            )
            .child(footer)
    }
}

#[cfg(test)]
mod tests {
    use super::selection_quad_bounds;
    use gpui::{Bounds, TestAppContext, point, px, size};

    use crate::showcase::BaseShowcase;

    #[gpui::test]
    fn text_selection_footer_stays_fixed_when_document_scrolls(cx: &mut TestAppContext) {
        let (view, window) =
            cx.add_window_view(|window, cx| BaseShowcase::new("text-selection", window, cx));
        window.update(|window, cx| window.draw(cx).clear(cx));

        let (footer_bounds, scroll) = view.read_with(window, |view, _| {
            (
                view.text_selection_footer_bounds
                    .borrow()
                    .expect("footer should be painted"),
                view.text_selection_scroll.clone(),
            )
        });
        scroll.set_offset(point(px(0.), px(-80.)));
        view.update(window, |_, cx| cx.notify());
        window.update(|window, cx| window.draw(cx).clear(cx));

        let scrolled_footer_bounds = view.read_with(window, |view, _| {
            view.text_selection_footer_bounds
                .borrow()
                .expect("footer should be painted after scrolling")
        });
        assert_eq!(scrolled_footer_bounds, footer_bounds);
    }

    #[test]
    fn wrapped_selection_paints_full_width_middle_lines() {
        let bounds = Bounds::new(point(px(10.), px(20.)), size(px(100.), px(100.)));
        let quads = selection_quad_bounds(
            point(px(40.), px(20.)),
            point(px(30.), px(80.)),
            bounds,
            px(20.),
        );

        assert_eq!(
            quads,
            vec![
                Bounds::from_corners(point(px(40.), px(20.)), point(px(110.), px(40.))),
                Bounds::from_corners(point(px(10.), px(40.)), point(px(110.), px(80.))),
                Bounds::from_corners(point(px(10.), px(80.)), point(px(30.), px(100.))),
            ]
        );
    }
}
