//! A plain `StyledText` renderer that participates in base window selection.
//!
//! Run with `cargo run -p gpui-base --example selectable_text`, then drag over
//! the sentence and copy it with the platform copy shortcut.

use gpui::{
    App, AppContext as _, BorderStyle, Bounds, Context, Corners, Edges, Element, ElementId,
    GlobalElementId, Hitbox, InspectorElementId, IntoElement, LayoutId, PaintQuad,
    ParentElement as _, Pixels, Point, Render, SharedString, Styled as _, StyledText, Window,
    WindowOptions, div, transparent_black,
};
use gpui_base::{
    SelectionRegionFrame, SelectionRunFrame, SelectionScopeId, TextSelection, TextSelectionRegion,
    WindowTextSelection as _,
};

struct PlainSelectableText {
    region: TextSelectionRegion,
    text: SharedString,
    styled_text: StyledText,
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
    fn new(region: TextSelectionRegion, text: impl Into<SharedString>) -> Self {
        let text = text.into();
        Self {
            region,
            styled_text: StyledText::new(text.clone()),
            text,
        }
    }

    fn paint_selection(
        layout: &gpui::TextLayout,
        range: std::ops::Range<usize>,
        window: &mut Window,
    ) {
        let Some(start) = layout.position_for_index(range.start) else {
            return;
        };
        let Some(end) = layout.position_for_index(range.end) else {
            return;
        };
        let line_height = layout.line_height();
        let bounds = layout.bounds();
        let color = gpui::hsla(0.58, 0.85, 0.62, 0.35);
        let paint = |bounds: Bounds<Pixels>, window: &mut Window| {
            window.paint_quad(PaintQuad {
                bounds,
                background: color.into(),
                corner_radii: Corners::default(),
                border_widths: Edges::default(),
                border_color: transparent_black(),
                border_style: BorderStyle::default(),
            });
        };

        for bounds in selection_quad_bounds(start, end, bounds, line_height) {
            paint(bounds, window);
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
        window.register_text_selection_region(
            self.region.clone(),
            SelectionRegionFrame {
                hitbox: hitbox.clone(),
                bounds,
                scroll_offset: Point::default(),
                scope: SelectionScopeId::default(),
                document_order: 0,
                text_bounds: vec![bounds],
            },
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
        let states = self.region.state().update(cx, |state, _| {
            state.project_selection_runs(&[SelectionRunFrame {
                order: 0,
                text: self.text.clone(),
                layout: layout.clone(),
                bounds,
            }])
        });
        if let Some(range) = states.into_iter().next().and_then(|state| state.byte_range) {
            Self::paint_selection(&layout, range, window);
        }
        self.styled_text
            .paint(id, inspector_id, bounds, &mut (), &mut (), window, cx);
    }
}

struct Example {
    region: TextSelectionRegion,
}

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let region = TextSelectionRegion::new("", cx);
        region.state().update(cx, |state, _| {
            state.on_selection(|_, cx| cx.refresh_windows());
        });
        Self { region }
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_8()
            .bg(gpui::white())
            .child(TextSelection)
            .child(PlainSelectableText::new(
                self.region.clone(),
                "Plain GPUI text — drag to select this UTF-8 text: café 🙂",
            ))
    }
}

fn main() {
    gpui_platform::application().run(|cx| {
        gpui_base::init(cx);
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(Example::new))
            .expect("failed to open selectable text example window");
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests {
    use super::selection_quad_bounds;
    use gpui::{Bounds, point, px, size};

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
