use std::rc::Rc;

use gpui::{
    canvas, deferred, div, px, App, AppContext as _, Bounds, Context, Empty, Entity,
    InteractiveElement, IntoElement, ParentElement as _, Pixels, Point, Render, Styled, Window,
};

use crate::{
    highlighter::LanguageRegistry,
    input::{InputState, Marker},
    text::TextView,
    ActiveTheme as _,
};

pub struct DiagnosticPopover {
    state: Entity<InputState>,
    pub(super) marker: Rc<Marker>,
    bounds: Bounds<Pixels>,
    open: bool,
}

impl DiagnosticPopover {
    pub fn new(marker: &Marker, state: Entity<InputState>, cx: &mut App) -> Entity<Self> {
        let marker = Rc::new(marker.clone());

        cx.new(|_| Self {
            marker,
            state,
            bounds: Bounds::default(),
            open: true,
        })
    }

    fn origin(&self, cx: &App) -> Option<Point<Pixels>> {
        let Some(range) = self.marker.range.as_ref() else {
            return None;
        };

        let (_, _, start_pos) = self
            .state
            .read(cx)
            .line_and_position_for_offset(range.start);

        start_pos
    }

    pub(super) fn show(&mut self, cx: &mut Context<Self>) {
        self.open = true;
        cx.notify();
    }

    pub(super) fn hide(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        cx.notify();
    }

    pub(super) fn check_to_hide(&mut self, mouse_position: Point<Pixels>, cx: &mut Context<Self>) {
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
        let theme = LanguageRegistry::global(cx).theme(cx.theme().is_dark());

        let message = self.marker.message.clone();
        let Some(pos) = self.origin(cx) else {
            return Empty.into_any_element();
        };
        let (border, bg, fg) = (
            self.marker.severity.border(theme),
            self.marker.severity.bg(theme),
            self.marker.severity.fg(theme),
        );

        let scroll_origin = self.state.read(cx).scroll_handle.offset();

        let y = pos.y - self.bounds.size.height + scroll_origin.y;
        let x = pos.x + scroll_origin.x;
        let max_width = px(500.).min(window.bounds().size.width - x);

        deferred(
            div()
                .id("code-editor-diagnostic-popover")
                .absolute()
                .left(x)
                .top(y)
                .px_1()
                .py_0p5()
                .text_xs()
                .bg(bg)
                .w(max_width)
                .text_color(fg)
                .border_1()
                .border_color(border)
                .rounded(cx.theme().radius)
                .shadow_sm()
                .child(TextView::markdown("message", message))
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
