use std::rc::Rc;

use gpui::{
    canvas, deferred, div, px, App, AppContext as _, Bounds, Context, Empty, Entity,
    InteractiveElement, IntoElement, ParentElement as _, Pixels, Point, Render, Styled, Window,
};

use crate::{highlighter::DiagnosticEntry, input::InputState, text::TextView, ActiveTheme as _};

pub struct DiagnosticPopover {
    state: Entity<InputState>,
    pub(crate) diagnostic: Rc<DiagnosticEntry>,
    bounds: Bounds<Pixels>,
    open: bool,
}

impl DiagnosticPopover {
    pub fn new(
        diagnostic: &DiagnosticEntry,
        state: Entity<InputState>,
        cx: &mut App,
    ) -> Entity<Self> {
        let diagnostic = Rc::new(diagnostic.clone());

        cx.new(|_| Self {
            diagnostic,
            state,
            bounds: Bounds::default(),
            open: true,
        })
    }

    fn origin(&self, cx: &App) -> Option<Point<Pixels>> {
        let state = self.state.read(cx);
        let Some(last_layout) = state.last_layout.as_ref() else {
            return None;
        };

        let line_number_width = last_layout.line_number_width;
        let (_, _, start_pos) = state.line_and_position_for_offset(self.diagnostic.range.start);

        start_pos.map(|pos| pos + Point::new(line_number_width, px(0.)))
    }

    pub(crate) fn show(&mut self, cx: &mut Context<Self>) {
        self.open = true;
        cx.notify();
    }

    pub(crate) fn hide(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        cx.notify();
    }

    pub(crate) fn check_to_hide(&mut self, mouse_position: Point<Pixels>, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }

        let padding = px(5.);
        let bounds = Bounds {
            origin: self.bounds.origin.map(|v| v - padding),
            size: self.bounds.size.map(|v| v + padding * 2.),
        };

        if !bounds.contains(&mouse_position) {
            self.hide(cx);
        }
    }
}

impl Render for DiagnosticPopover {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        if !self.open {
            return Empty.into_any_element();
        }

        let view = cx.entity();

        let message = self.diagnostic.message.clone();
        let Some(pos) = self.origin(cx) else {
            return Empty.into_any_element();
        };
        let (border, bg, fg) = (
            self.diagnostic.severity.border(cx),
            self.diagnostic.severity.bg(cx),
            self.diagnostic.severity.fg(cx),
        );

        let scroll_origin = self.state.read(cx).scroll_handle.offset();

        let y = pos.y - self.bounds.size.height + scroll_origin.y;
        let x = pos.x + scroll_origin.x;
        let max_width = px(500.).min(window.bounds().size.width - x);

        deferred(
            div()
                .id("diagnostic-popover")
                .absolute()
                .left(x)
                .top(y)
                .px_1()
                .py_0p5()
                .text_xs()
                .max_w(max_width)
                .bg(bg)
                .text_color(fg)
                .border_1()
                .border_color(border)
                .rounded(cx.theme().radius)
                .shadow_md()
                .child(TextView::markdown("message", message, window, cx).selectable())
                .child(
                    canvas(
                        move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                        |_, _, _, _| {},
                    )
                    .top_0()
                    .left_0()
                    .absolute()
                    .size_full(),
                )
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.open = false;
                    cx.notify();
                })),
        )
        .into_any_element()
    }
}
